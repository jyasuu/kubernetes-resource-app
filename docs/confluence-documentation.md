# Kubernetes Custom Resource Controller in Rust - Technical Documentation

## Overview

This document provides comprehensive technical documentation for our production-ready Kubernetes Custom Resource Controller implemented in Rust. The controller manages `MyApp` custom resources and demonstrates industry best practices for Kubernetes operators.

## Architecture

### High-Level Design

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   kubectl/API   │────│  MyApp CRD       │────│  Controller     │
│                 │    │  (Custom Resource│    │  (Rust)         │
└─────────────────┘    │   Definition)    │    └─────────────────┘
                       └──────────────────┘              │
                                 │                       │
                                 │                       ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │  Admission       │    │  Child Resources│
                       │  Webhooks        │    │  - Deployment   │
                       │  - Validation    │    │  - Service      │
                       │  - Mutation      │    └─────────────────┘
                       └──────────────────┘
```

### Component Breakdown

#### 1. Custom Resource Definition (CRD)
- **Group**: `example.com`
- **Version**: `v1`
- **Kind**: `MyApp`
- **Scope**: Namespaced
- **Features**:
  - Status subresource
  - Schema validation
  - Custom print columns
  - Short names (`ma`)

#### 2. Controller Logic
- **Reconciliation Loop**: Watches for MyApp resources and ensures desired state
- **Error Handling**: Exponential backoff with typed errors
- **Status Management**: Updates status with conditions and observed generation
- **Child Resource Management**: Creates and manages Deployments and Services

#### 3. Admission Control
- **Validating Webhook**: Prevents invalid configurations
- **Mutating Webhook**: Adds default values and labels

## Implementation Details

### Key Data Structures

```rust
// Main Custom Resource
pub struct MyAppSpec {
    pub replicas: i32,           // 1-100 replicas
    pub image: String,           // Container image (no 'latest' allowed)
    pub env_vars: BTreeMap<String, String>,  // Environment variables
    pub resources: Option<ResourceRequirements>, // CPU/Memory limits
}

// Status tracking
pub struct MyAppStatus {
    pub state: String,           // Current state
    pub observed_generation: Option<i64>, // For drift detection
    pub conditions: Vec<Condition>,       // Kubernetes-style conditions
    pub last_updated: Option<String>,     // Timestamp
}
```

### Best Practices Implemented

#### 1. **Finalizers**
- Pattern: `myapps.example.com/finalizer`
- Ensures proper cleanup of child resources before deletion
- Prevents data loss and orphaned resources

```rust
const FINALIZER: &str = "myapps.example.com/finalizer";

// Added on creation, removed after cleanup
pub async fn add_finalizer(myapp: &MyApp, client: Client) -> Result<MyApp, kube::Error>
pub async fn remove_finalizer(myapp: &MyApp, client: Client) -> Result<MyApp, kube::Error>
```

#### 2. **Owner References**
- Establishes parent-child relationships
- Enables automatic garbage collection
- Sets `controller: true` and `blockOwnerDeletion: true`

```rust
pub fn create_owner_reference(myapp: &MyApp) -> OwnerReference {
    OwnerReference {
        api_version: MyApp::api_version(&()).to_string(),
        kind: MyApp::kind(&()).to_string(),
        name: myapp.name_any(),
        uid: myapp.metadata.uid.clone().unwrap(),
        controller: Some(true),
        block_owner_deletion: Some(true),
    }
}
```

#### 3. **Status Conditions**
- Follows Kubernetes conventions
- Provides detailed state information
- Includes timestamps and reasons

```rust
impl Condition {
    pub fn ready(status: bool, reason: &str, message: &str) -> Self {
        Self {
            r#type: "Ready".to_string(),
            status: if status { "True" } else { "False" }.to_string(),
            reason: reason.to_string(),
            message: message.to_string(),
            last_transition_time: chrono::Utc::now().to_rfc3339(),
        }
    }
}
```

#### 4. **Validation Layers**

**Schema Validation (CRD Level)**:
```rust
#[schemars(range(min = 1, max = 100))]
pub replicas: i32,

#[schemars(regex(pattern = r"^[a-z0-9-./]+:[a-z0-9.-]+$"))]
pub image: String,
```

**Custom Validation (Application Level)**:
```rust
pub fn validate(&self) -> Result<(), String> {
    if self.spec.replicas < 1 || self.spec.replicas > 100 {
        return Err("replicas must be between 1 and 100".to_string());
    }
    // Additional validation logic...
}
```

**Webhook Validation (Admission Control)**:
```rust
// Prevents 'latest' image tags
if myapp.spec.image.contains("latest") {
    return Ok(warp::reply::json(&AdmissionResponse::invalid(
        "Image tag 'latest' is not allowed".to_string()
    ).into_review()));
}
```

## Deployment Guide

### Prerequisites
- Kubernetes cluster (1.16+)
- Docker for building images
- Rust 1.75+ for development
- `kubectl` configured for cluster access

### Deployment Steps

1. **Build and Generate CRD**:
```bash
cargo build --release
cargo run -- generate-crd
kubectl apply -f crd.yaml
```

2. **Set up RBAC**:
```bash
./scripts/deploy.sh  # Includes RBAC setup
```

3. **Deploy Controller**:
```bash
docker build -t myapp-controller:latest .
kubectl apply -f deploy/controller-deployment.yaml
```

4. **Optional: Deploy Webhooks**:
```bash
./scripts/generate-webhook-certs.sh
kubectl apply -f deploy/webhook-manifests.yaml
```

### Verification

```bash
# Check CRD installation
kubectl get crds myapps.example.com

# Test with sample resource
kubectl apply -f examples/sample-myapp.yaml
kubectl get myapps
kubectl describe myapp sample-app

# Verify child resources
kubectl get deployments,services -l managed-by=myapp-controller
```

## Operations Guide

### Monitoring and Observability

#### Logs
```bash
# Controller logs
kubectl logs -f deployment/myapp-controller

# Look for these patterns:
# - "Reconciling MyApp namespace/name"
# - "Finalizer added/removed"
# - "Created deployment/service with owner reference"
```

#### Status Checking
```bash
# Check MyApp status
kubectl get myapp sample-app -o jsonpath='{.status}'

# Check conditions
kubectl get myapp sample-app -o jsonpath='{.status.conditions[*]}'
```

#### Metrics (Future Enhancement)
- Reconciliation duration
- Error rates
- Resource creation/deletion counts
- Queue depth

### Troubleshooting

#### Common Issues

1. **CRD Not Found**
   - Ensure CRD is applied: `kubectl get crds myapps.example.com`
   - Check CRD generation: `cargo run -- generate-crd`

2. **RBAC Errors**
   - Verify service account: `kubectl get sa myapp-controller`
   - Check cluster role binding: `kubectl get clusterrolebinding myapp-controller`

3. **Reconciliation Failures**
   - Check controller logs: `kubectl logs deployment/myapp-controller`
   - Verify resource status: `kubectl describe myapp <name>`

4. **Webhook Failures**
   - Check certificate validity: `kubectl get secret myapp-webhook-certs`
   - Verify webhook configuration: `kubectl get validatingwebhookconfiguration`

#### Debug Commands
```bash
# Get detailed events
kubectl describe myapp <name>

# Check child resources
kubectl get all -l app=<myapp-name>

# Validate webhook response
kubectl apply --dry-run=server -f examples/sample-myapp.yaml
```

## Development Guide

### Local Development

1. **Setup Environment**:
```bash
# Install dependencies
cargo check

# Run locally (requires kubeconfig)
RUST_LOG=info cargo run
```

2. **Testing**:
```bash
# Unit tests
cargo test

# Integration tests
./scripts/test-controller.sh
```

3. **Code Structure**:
```
src/main.rs:
├── Custom Resource Definition (MyApp, MyAppSpec, MyAppStatus)
├── Validation Logic
├── Finalizer Management
├── Owner Reference Handling
├── Admission Webhooks (Validate/Mutate)
├── Controller Logic (Reconcile function)
└── Main Entry Point
```

### Contributing Guidelines

1. **Code Style**: Follow `rustfmt` formatting
2. **Error Handling**: Use typed errors with `thiserror`
3. **Logging**: Use structured logging with appropriate levels
4. **Testing**: Add tests for new functionality
5. **Documentation**: Update README and inline documentation

## Security Considerations

### RBAC Permissions
- **Minimal Permissions**: Only necessary verbs on required resources
- **Namespace Scoped**: Where possible, avoid cluster-wide permissions
- **Service Account**: Dedicated service account for controller

### Webhook Security
- **TLS Certificates**: Properly generated and rotated certificates
- **Admission Review**: Validate all incoming requests
- **Timeout Handling**: Appropriate timeouts to prevent hanging

### Image Security
- **No Latest Tags**: Enforced via validation webhook
- **Base Image**: Uses minimal Debian base for smaller attack surface
- **Non-Root User**: Container runs as non-root user

## Performance Considerations

### Controller Performance
- **Exponential Backoff**: Prevents thundering herd on errors
- **Requeue Strategy**: 5-minute periodic reconciliation
- **Resource Watching**: Efficient event-driven reconciliation

### Resource Limits
- **Memory**: ~50MB typical usage
- **CPU**: Minimal CPU usage during steady state
- **Network**: Low network usage, only API calls

## Future Enhancements

### Planned Features
1. **Metrics and Monitoring**
   - Prometheus metrics
   - Grafana dashboards
   - Alerting rules

2. **Advanced Scheduling**
   - Node affinity rules
   - Pod anti-affinity
   - Resource quotas

3. **Multi-tenancy**
   - Namespace isolation
   - RBAC integration
   - Resource limits per tenant

4. **High Availability**
   - Leader election
   - Multiple controller replicas
   - Graceful shutdown

### Extension Points
- **Custom Validation**: Add domain-specific validation rules
- **Additional Resources**: Manage ConfigMaps, Secrets, etc.
- **External Integrations**: Connect to external systems
- **Custom Schedulers**: Implement advanced scheduling logic

## References

- [Kubernetes Custom Resources](https://kubernetes.io/docs/concepts/extend-kubernetes/api-extension/custom-resources/)
- [Controller Pattern](https://kubernetes.io/docs/concepts/architecture/controller/)
- [Admission Controllers](https://kubernetes.io/docs/reference/access-authn-authz/admission-controllers/)
- [Kube-rs Documentation](https://docs.rs/kube/latest/kube/)
- [Operator Best Practices](https://sdk.operatorframework.io/docs/best-practices/)

---

**Document Version**: 1.0  
**Last Updated**: {current_date}  
**Maintainer**: DevOps Team  
**Review Schedule**: Quarterly