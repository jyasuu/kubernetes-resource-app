# Kubernetes Custom Resource Controller in Rust

A production-ready Kubernetes operator implementation in Rust demonstrating best practices for Custom Resource Definitions (CRDs), controllers, finalizers, owner references, and admission webhooks.

## Features

✅ **Custom Resource Definition (CRD)** with status subresource  
✅ **Finalizers** for proper cleanup before deletion  
✅ **Owner References** for automatic garbage collection  
✅ **Validating Webhooks** for admission control  
✅ **Mutating Webhooks** for setting default values  
✅ **Status Conditions** following Kubernetes conventions  
✅ **Schema Validation** using JSON Schema  
✅ **Custom Print Columns** for `kubectl get` output  
✅ **Proper Error Handling** with exponential backoff  
✅ **Observability** with generation tracking  

## Architecture

The controller implements a complete Kubernetes operator pattern:

- **MyApp CRD**: Defines applications with replicas, image, environment variables, and resource requirements
- **Controller**: Reconciles MyApp resources by creating/managing Deployments and Services
- **Webhooks**: Validates and mutates resources before they're stored in etcd
- **Finalizers**: Ensures proper cleanup when resources are deleted
- **Owner References**: Automatic cleanup of child resources

## Quick Start

### 1. Build the Controller

```bash
# Build the Rust application
cargo build --release

# Or build Docker image
docker build -t myapp-controller:latest .
```

### 2. Generate and Apply CRD

```bash
# Generate the CRD YAML file
cargo run -- generate-crd

# Apply the CRD to your cluster
kubectl apply -f crd.yaml
```

### 3. Run the Controller

```bash
# Run locally (requires kubeconfig)
cargo run

# Or deploy to cluster using the Dockerfile
```

### 4. Create a Sample Resource

```bash
# Apply the example resource
kubectl apply -f examples/sample-myapp.yaml

# Check the resource
kubectl get myapps
kubectl describe myapp sample-app
```

## Usage

### Command Line Options

```bash
# Run the controller (default)
./myapp-controller

# Generate CRD YAML
./myapp-controller generate-crd

# Run webhook server
./myapp-controller webhook
```

### Custom Resource Example

```yaml
apiVersion: example.com/v1
kind: MyApp
metadata:
  name: my-app
  namespace: default
spec:
  replicas: 3
  image: nginx:1.21.0
  envVars:
    ENV: production
    LOG_LEVEL: info
  resources:
    cpu: "500m"
    memory: "512Mi"
```

### Viewing Resources

```bash
# List all MyApp resources with custom columns
kubectl get myapps
# or using the short name
kubectl get ma

# Get detailed information
kubectl describe myapp my-app

# Check status
kubectl get myapp my-app -o jsonpath='{.status}'
```

## Best Practices Implemented

### 1. Status Subresource
- Separate status tracking from spec
- Conditions following Kubernetes conventions
- Observed generation for change detection

### 2. Validation
- JSON Schema validation at API level
- Custom validation logic in webhooks
- Proper error messages and responses

### 3. Finalizers
- Prevents deletion until cleanup is complete
- Proper resource cleanup (Deployments, Services)
- Safe removal of finalizer after cleanup

### 4. Owner References
- Automatic garbage collection of child resources
- Proper parent-child relationships
- Controller and blockOwnerDeletion flags

### 5. Observability
- Structured logging
- Status conditions tracking
- Generation-based reconciliation

### 6. Error Handling
- Typed errors with proper propagation
- Exponential backoff on failures
- Graceful degradation

## Webhook Setup

### 1. Generate Certificates

```bash
# Generate TLS certificates for webhooks
./scripts/generate-webhook-certs.sh
```

### 2. Deploy Webhook Server

```bash
# Deploy the webhook server and configurations
kubectl apply -f deploy/webhook-manifests.yaml
```

### 3. Test Webhooks

```bash
# This should be rejected (uses 'latest' tag)
kubectl apply -f - <<EOF
apiVersion: example.com/v1
kind: MyApp
metadata:
  name: test-app
spec:
  replicas: 1
  image: nginx:latest
EOF

# This should succeed and get default labels added
kubectl apply -f examples/sample-myapp.yaml
```

## Development

### Project Structure

```
├── src/
│   └── main.rs              # Main controller implementation
├── examples/
│   └── sample-myapp.yaml    # Example resource
├── deploy/
│   └── webhook-manifests.yaml # Webhook deployment
├── scripts/
│   └── generate-webhook-certs.sh # Certificate generation
├── Dockerfile               # Container build
├── Cargo.toml              # Rust dependencies
└── README.md               # This file
```

### Key Components

- **MyApp/MyAppSpec**: The custom resource definition
- **MyAppStatus**: Status subresource with conditions
- **Finalizers**: Cleanup logic before deletion
- **Owner References**: Parent-child relationships
- **Webhooks**: Admission control (validation/mutation)
- **Controller**: Main reconciliation loop

### Testing

```bash
# Check that the code compiles
cargo check

# Run with logging
RUST_LOG=info cargo run

# Test CRD generation
cargo run -- generate-crd
```

## Deployment to Kubernetes

### 1. Build and Push Image

```bash
# Build the Docker image
docker build -t your-registry/myapp-controller:v1.0.0 .

# Push to your registry
docker push your-registry/myapp-controller:v1.0.0
```

### 2. Create RBAC

```bash
kubectl apply -f - <<EOF
apiVersion: v1
kind: ServiceAccount
metadata:
  name: myapp-controller
  namespace: default
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: myapp-controller
rules:
- apiGroups: ["example.com"]
  resources: ["myapps"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["example.com"]
  resources: ["myapps/status"]
  verbs: ["get", "update", "patch"]
- apiGroups: ["apps"]
  resources: ["deployments"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: [""]
  resources: ["services"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: myapp-controller
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: myapp-controller
subjects:
- kind: ServiceAccount
  name: myapp-controller
  namespace: default
EOF
```

### 3. Deploy Controller

```bash
kubectl apply -f - <<EOF
apiVersion: apps/v1
kind: Deployment
metadata:
  name: myapp-controller
  namespace: default
spec:
  replicas: 1
  selector:
    matchLabels:
      app: myapp-controller
  template:
    metadata:
      labels:
        app: myapp-controller
    spec:
      serviceAccountName: myapp-controller
      containers:
      - name: controller
        image: your-registry/myapp-controller:v1.0.0
        env:
        - name: RUST_LOG
          value: info
EOF
```

## Troubleshooting

### Common Issues

1. **CRD not found**: Make sure to apply the CRD first
2. **RBAC errors**: Check that the service account has proper permissions
3. **Webhook failures**: Verify certificates and network connectivity
4. **Reconciliation loops**: Check for proper status updates

### Debugging

```bash
# Check controller logs
kubectl logs -f deployment/myapp-controller

# Check webhook logs
kubectl logs -f deployment/myapp-webhook

# Describe resources for events
kubectl describe myapp sample-app
kubectl describe deployment sample-app-deployment
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.


