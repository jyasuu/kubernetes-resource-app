I'll help you implement best practices for Kubernetes Custom Resources in Rust. Let me create a comprehensive example using the `kube` crate ecosystem.I've created a comprehensive example showing best practices for Kubernetes Custom Resources in Rust. Here are the key practices demonstrated:

## Best Practices Included:

1. **Status Subresource**: Separate `MyAppStatus` struct for tracking reconciliation state, conditions, and observed generation

2. **Validation**: 
   - Schema validation using `schemars` attributes (ranges, regex patterns)
   - Custom validation methods on the resource

3. **Conditions**: Standard Kubernetes condition pattern for tracking resource health

4. **Print Columns**: Custom columns for `kubectl get` output

5. **Proper Error Handling**: Using `thiserror` for typed errors with proper error propagation

6. **Reconciliation Loop**:
   - Check for deletion timestamp (finalizer pattern)
   - Update status subresource separately
   - Implement exponential backoff on errors
   - Periodic requeue for drift detection

7. **Observability**: Tracking `observedGeneration` to detect when spec changes

8. **Naming Conventions**: Using `camelCase` serialization to match Kubernetes conventions

9. **Resource Scoping**: Properly namespaced resource with shortnames

10. **Type Safety**: Using BTreeMap for optional fields, proper Option<> usage

## To deploy this:

1. Generate CRD YAML: Use `kube::CustomResourceExt::crd()` to output the CRD definition
2. Apply to cluster: `kubectl apply -f crd.yaml`
3. Run controller: Build and deploy your operator

Would you like me to add examples for finalizers, webhooks, or owner references?

```rust
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Define your Custom Resource with proper derive macros
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    group = "example.com",
    version = "v1",
    kind = "MyApp",
    namespaced,
    status = "MyAppStatus",
    shortname = "ma",
    printcolumn = r#"{"name":"State", "type":"string", "jsonPath":".status.state"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct MyAppSpec {
    /// Number of replicas desired
    #[schemars(range(min = 1, max = 100))]
    pub replicas: i32,
    
    /// Image to deploy
    #[schemars(regex(pattern = r"^[a-z0-9-./]+:[a-z0-9.-]+$"))]
    pub image: String,
    
    /// Optional environment variables
    #[serde(default)]
    pub env_vars: BTreeMap<String, String>,
    
    /// Resource requirements
    #[serde(default)]
    pub resources: Option<ResourceRequirements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    pub cpu: String,
    pub memory: String,
}

// Status subresource - best practice for tracking reconciliation state
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyAppStatus {
    /// Current state of the application
    pub state: String,
    
    /// Observed generation
    #[serde(default)]
    pub observed_generation: Option<i64>,
    
    /// Conditions tracking various aspects of the resource
    #[serde(default)]
    pub conditions: Vec<Condition>,
    
    /// Last update timestamp
    #[serde(default)]
    pub last_updated: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub r#type: String,
    pub status: String,
    pub reason: String,
    pub message: String,
    pub last_transition_time: String,
}

// Validation methods
impl MyApp {
    /// Validate the spec before processing
    pub fn validate(&self) -> Result<(), String> {
        if self.spec.replicas < 1 || self.spec.replicas > 100 {
            return Err("replicas must be between 1 and 100".to_string());
        }
        
        if self.spec.image.is_empty() {
            return Err("image cannot be empty".to_string());
        }
        
        Ok(())
    }
    
    /// Check if resource needs reconciliation
    pub fn needs_reconciliation(&self) -> bool {
        self.status.as_ref()
            .and_then(|s| s.observed_generation)
            .map(|og| og != self.metadata.generation.unwrap_or(0))
            .unwrap_or(true)
    }
}

// Helper to create conditions
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

// Example controller reconciliation function
use kube::{
    api::{Api, Patch, PatchParams},
    runtime::controller::{Action, Controller},
    Client, ResourceExt,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReconcileError {
    #[error("Kube error: {0}")]
    KubeError(#[from] kube::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Finalizer error: {0}")]
    FinalizerError(String),
}

pub struct Context {
    pub client: Client,
}

pub async fn reconcile(myapp: Arc<MyApp>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    let ns = myapp.namespace().unwrap();
    let name = myapp.name_any();
    let api: Api<MyApp> = Api::namespaced(ctx.client.clone(), &ns);
    
    // Validate the resource
    myapp.validate()
        .map_err(ReconcileError::ValidationError)?;
    
    // Check if resource is being deleted
    if myapp.metadata.deletion_timestamp.is_some() {
        return handle_deletion(myapp, api).await;
    }
    
    // Main reconciliation logic here
    println!("Reconciling MyApp {}/{}", ns, name);
    
    // Update status subresource
    let new_status = MyAppStatus {
        state: "Running".to_string(),
        observed_generation: myapp.metadata.generation,
        conditions: vec![
            Condition::ready(true, "ReconcileSuccess", "Resource reconciled successfully")
        ],
        last_updated: Some(chrono::Utc::now().to_rfc3339()),
    };
    
    let status_patch = serde_json::json!({
        "status": new_status
    });
    
    api.patch_status(
        &name,
        &PatchParams::default(),
        &Patch::Merge(&status_patch)
    ).await?;
    
    // Requeue after 5 minutes for periodic reconciliation
    Ok(Action::requeue(std::time::Duration::from_secs(300)))
}

async fn handle_deletion(
    myapp: Arc<MyApp>,
    api: Api<MyApp>,
) -> Result<Action, ReconcileError> {
    // Cleanup logic here
    println!("Cleaning up MyApp {}", myapp.name_any());
    
    // Remove finalizer when cleanup is done
    // (Finalizer management code would go here)
    
    Ok(Action::await_change())
}

pub fn error_policy(
    _myapp: Arc<MyApp>,
    error: &ReconcileError,
    _ctx: Arc<Context>,
) -> Action {
    eprintln!("Reconciliation error: {:?}", error);
    // Exponential backoff: retry after 60 seconds
    Action::requeue(std::time::Duration::from_secs(60))
}

// Example main function to run the controller
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let context = Arc::new(Context {
        client: client.clone(),
    });
    
    let myapps = Api::<MyApp>::all(client);
    
    Controller::new(myapps, Default::default())
        .run(reconcile, error_policy, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => println!("Reconciled: {:?}", o),
                Err(e) => eprintln!("Reconcile error: {:?}", e),
            }
        })
        .await;
    
    Ok(())
}

// Cargo.toml dependencies needed:
// [dependencies]
// kube = { version = "0.95", features = ["runtime", "derive"] }
// k8s-openapi = { version = "0.23", features = ["latest"] }
// schemars = "0.8"
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// tokio = { version = "1.0", features = ["full"] }
// thiserror = "1.0"
// chrono = "0.4"
```

I've created the CRD YAML that corresponds to the Rust code. Here's how to use it:

## Generate CRD from Rust Code

You can also generate this programmatically. Add this to your Rust code:

```rust
use std::fs::File;
use std::io::Write;

fn main() {
    let crd = MyApp::crd();
    let yaml = serde_yaml::to_string(&crd).unwrap();
    
    let mut file = File::create("crd.yaml").unwrap();
    file.write_all(yaml.as_bytes()).unwrap();
    
    println!("CRD written to crd.yaml");
}
```

## Deploy the CRD

```bash
# Apply the CRD to your cluster
kubectl apply -f crd.yaml

# Verify it was created
kubectl get crds myapps.example.com

# Check the schema
kubectl explain myapp.spec
```

## Create a Sample Resource

```yaml
apiVersion: example.com/v1
kind: MyApp
metadata:
  name: my-sample-app
  namespace: default
spec:
  replicas: 3
  image: nginx:1.21
  envVars:
    ENV: production
    LOG_LEVEL: info
  resources:
    cpu: "500m"
    memory: "512Mi"
```

## View Resources

```bash
# List all MyApp resources with custom columns
kubectl get myapps
# or using the short name
kubectl get ma

# Get detailed info
kubectl describe myapp my-sample-app
```

The CRD includes validation, so invalid resources will be rejected at the API level before reaching your controller!


```yaml
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: myapps.example.com
spec:
  group: example.com
  names:
    categories: []
    kind: MyApp
    plural: myapps
    shortNames:
      - ma
    singular: myapp
  scope: Namespaced
  versions:
    - additionalPrinterColumns:
        - jsonPath: .status.state
          name: State
          type: string
        - jsonPath: .metadata.creationTimestamp
          name: Age
          type: date
      name: v1
      schema:
        openAPIV3Schema:
          description: Auto-generated derived type for MyAppSpec via `CustomResource`
          properties:
            spec:
              properties:
                envVars:
                  additionalProperties:
                    type: string
                  default: {}
                  description: Optional environment variables
                  type: object
                image:
                  description: Image to deploy
                  pattern: ^[a-z0-9-./]+:[a-z0-9.-]+$
                  type: string
                replicas:
                  description: Number of replicas desired
                  format: int32
                  maximum: 100
                  minimum: 1
                  type: integer
                resources:
                  description: Resource requirements
                  nullable: true
                  properties:
                    cpu:
                      type: string
                    memory:
                      type: string
                  required:
                    - cpu
                    - memory
                  type: object
              required:
                - image
                - replicas
              type: object
            status:
              nullable: true
              properties:
                conditions:
                  default: []
                  description: Conditions tracking various aspects of the resource
                  items:
                    properties:
                      lastTransitionTime:
                        type: string
                      message:
                        type: string
                      reason:
                        type: string
                      status:
                        type: string
                      type:
                        type: string
                    required:
                      - lastTransitionTime
                      - message
                      - reason
                      - status
                      - type
                    type: object
                  type: array
                lastUpdated:
                  description: Last update timestamp
                  nullable: true
                  type: string
                observedGeneration:
                  default: null
                  description: Observed generation
                  format: int64
                  nullable: true
                  type: integer
                state:
                  description: Current state of the application
                  type: string
              required:
                - state
              type: object
          required:
            - spec
          title: MyApp
          type: object
      served: true
      storage: true
      subresources:
        status: {}
```


## v2

```rust
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Define your Custom Resource with proper derive macros
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    group = "example.com",
    version = "v1",
    kind = "MyApp",
    namespaced,
    status = "MyAppStatus",
    shortname = "ma",
    printcolumn = r#"{"name":"State", "type":"string", "jsonPath":".status.state"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct MyAppSpec {
    /// Number of replicas desired
    #[schemars(range(min = 1, max = 100))]
    pub replicas: i32,
    
    /// Image to deploy
    #[schemars(regex(pattern = r"^[a-z0-9-./]+:[a-z0-9.-]+$"))]
    pub image: String,
    
    /// Optional environment variables
    #[serde(default)]
    pub env_vars: BTreeMap<String, String>,
    
    /// Resource requirements
    #[serde(default)]
    pub resources: Option<ResourceRequirements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    pub cpu: String,
    pub memory: String,
}

// Status subresource - best practice for tracking reconciliation state
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyAppStatus {
    /// Current state of the application
    pub state: String,
    
    /// Observed generation
    #[serde(default)]
    pub observed_generation: Option<i64>,
    
    /// Conditions tracking various aspects of the resource
    #[serde(default)]
    pub conditions: Vec<Condition>,
    
    /// Last update timestamp
    #[serde(default)]
    pub last_updated: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub r#type: String,
    pub status: String,
    pub reason: String,
    pub message: String,
    pub last_transition_time: String,
}

// Validation methods
impl MyApp {
    /// Validate the spec before processing
    pub fn validate(&self) -> Result<(), String> {
        if self.spec.replicas < 1 || self.spec.replicas > 100 {
            return Err("replicas must be between 1 and 100".to_string());
        }
        
        if self.spec.image.is_empty() {
            return Err("image cannot be empty".to_string());
        }
        
        Ok(())
    }
    
    /// Check if resource needs reconciliation
    pub fn needs_reconciliation(&self) -> bool {
        self.status.as_ref()
            .and_then(|s| s.observed_generation)
            .map(|og| og != self.metadata.generation.unwrap_or(0))
            .unwrap_or(true)
    }
}

// Helper to create conditions
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

// ============================================================================
// FINALIZERS - Ensure cleanup before deletion
// ============================================================================

use kube::api::{Api, Patch, PatchParams, PostParams};
use kube::{Client, ResourceExt};

const FINALIZER: &str = "myapps.example.com/finalizer";

pub async fn add_finalizer(myapp: &MyApp, client: Client) -> Result<MyApp, kube::Error> {
    let api: Api<MyApp> = Api::namespaced(
        client,
        &myapp.namespace().unwrap()
    );
    
    let mut finalizers = myapp.finalizers().to_vec();
    if !finalizers.contains(&FINALIZER.to_string()) {
        finalizers.push(FINALIZER.to_string());
        
        let patch = serde_json::json!({
            "metadata": {
                "finalizers": finalizers
            }
        });
        
        api.patch(
            &myapp.name_any(),
            &PatchParams::default(),
            &Patch::Merge(&patch)
        ).await
    } else {
        Ok(myapp.clone())
    }
}

pub async fn remove_finalizer(myapp: &MyApp, client: Client) -> Result<MyApp, kube::Error> {
    let api: Api<MyApp> = Api::namespaced(
        client,
        &myapp.namespace().unwrap()
    );
    
    let mut finalizers = myapp.finalizers().to_vec();
    finalizers.retain(|f| f != FINALIZER);
    
    let patch = serde_json::json!({
        "metadata": {
            "finalizers": finalizers
        }
    });
    
    api.patch(
        &myapp.name_any(),
        &PatchParams::default(),
        &Patch::Merge(&patch)
    ).await
}

async fn cleanup_resources(myapp: &MyApp, client: Client) -> Result<(), Box<dyn std::error::Error>> {
    let ns = myapp.namespace().unwrap();
    println!("Cleaning up resources for MyApp {}/{}", ns, myapp.name_any());
    
    // Delete owned Deployments
    let deployments: Api<k8s_openapi::api::apps::v1::Deployment> = 
        Api::namespaced(client.clone(), &ns);
    
    let deploy_name = format!("{}-deployment", myapp.name_any());
    if deployments.get_opt(&deploy_name).await?.is_some() {
        deployments.delete(&deploy_name, &Default::default()).await?;
        println!("Deleted deployment: {}", deploy_name);
    }
    
    // Delete owned Services
    let services: Api<k8s_openapi::api::core::v1::Service> = 
        Api::namespaced(client.clone(), &ns);
    
    let svc_name = format!("{}-service", myapp.name_any());
    if services.get_opt(&svc_name).await?.is_some() {
        services.delete(&svc_name, &Default::default()).await?;
        println!("Deleted service: {}", svc_name);
    }
    
    Ok(())
}

// ============================================================================
// OWNER REFERENCES - Establish parent-child relationships
// ============================================================================

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    Container, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use std::collections::BTreeMap as StdBTreeMap;

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

pub async fn create_deployment(
    myapp: &MyApp,
    client: Client
) -> Result<Deployment, kube::Error> {
    let ns = myapp.namespace().unwrap();
    let name = format!("{}-deployment", myapp.name_any());
    let owner_ref = create_owner_reference(myapp);
    
    let mut labels = StdBTreeMap::new();
    labels.insert("app".to_string(), myapp.name_any());
    labels.insert("managed-by".to_string(), "myapp-controller".to_string());
    
    let deployment = Deployment {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            labels: Some(labels.clone()),
            owner_references: Some(vec![owner_ref]), // Set owner reference
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(myapp.spec.replicas),
            selector: k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                    labels: Some(labels.clone()),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "app".to_string(),
                        image: Some(myapp.spec.image.clone()),
                        env: Some(
                            myapp.spec.env_vars.iter()
                                .map(|(k, v)| k8s_openapi::api::core::v1::EnvVar {
                                    name: k.clone(),
                                    value: Some(v.clone()),
                                    ..Default::default()
                                })
                                .collect()
                        ),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    };
    
    let api: Api<Deployment> = Api::namespaced(client, &ns);
    api.create(&PostParams::default(), &deployment).await
}

pub async fn create_service(
    myapp: &MyApp,
    client: Client
) -> Result<Service, kube::Error> {
    let ns = myapp.namespace().unwrap();
    let name = format!("{}-service", myapp.name_any());
    let owner_ref = create_owner_reference(myapp);
    
    let mut labels = StdBTreeMap::new();
    labels.insert("app".to_string(), myapp.name_any());
    
    let service = Service {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            labels: Some(labels.clone()),
            owner_references: Some(vec![owner_ref]), // Set owner reference
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(labels),
            ports: Some(vec![ServicePort {
                port: 80,
                target_port: Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(80)),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };
    
    let api: Api<Service> = Api::namespaced(client, &ns);
    api.create(&PostParams::default(), &service).await
}

// ============================================================================
// ADMISSION WEBHOOKS - Validation and Mutation
// ============================================================================

use kube::core::admission::{AdmissionRequest, AdmissionResponse, AdmissionReview};
use warp::{Filter, Rejection, Reply};

// Validating Webhook
pub async fn validate_webhook(
    body: AdmissionReview<MyApp>
) -> Result<impl Reply, Rejection> {
    let req: AdmissionRequest<MyApp> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            eprintln!("Invalid admission request: {}", err);
            return Ok(warp::reply::json(&AdmissionResponse::invalid(
                format!("Invalid request: {}", err)
            ).into_review()));
        }
    };
    
    let myapp = match req.object {
        Some(obj) => obj,
        None => {
            return Ok(warp::reply::json(&AdmissionResponse::invalid(
                "No object in request".to_string()
            ).into_review()));
        }
    };
    
    // Validate the MyApp resource
    match myapp.validate() {
        Ok(_) => {
            // Additional custom validation
            if myapp.spec.image.contains("latest") {
                return Ok(warp::reply::json(&AdmissionResponse::invalid(
                    "Image tag 'latest' is not allowed".to_string()
                ).into_review()));
            }
            
            // Validation passed
            Ok(warp::reply::json(&AdmissionResponse::from(&req)
                .into_review()))
        }
        Err(e) => {
            Ok(warp::reply::json(&AdmissionResponse::invalid(e)
                .into_review()))
        }
    }
}

// Mutating Webhook
pub async fn mutate_webhook(
    body: AdmissionReview<MyApp>
) -> Result<impl Reply, Rejection> {
    let req: AdmissionRequest<MyApp> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            return Ok(warp::reply::json(&AdmissionResponse::invalid(
                format!("Invalid request: {}", err)
            ).into_review()));
        }
    };
    
    let mut myapp = match req.object {
        Some(obj) => obj,
        None => {
            return Ok(warp::reply::json(&AdmissionResponse::invalid(
                "No object in request".to_string()
            ).into_review()));
        }
    };
    
    // Add default labels if not present
    let labels = myapp.metadata.labels.get_or_insert_with(StdBTreeMap::new);
    labels.entry("app.kubernetes.io/managed-by".to_string())
        .or_insert("myapp-controller".to_string());
    
    // Set default resource limits if not specified
    if myapp.spec.resources.is_none() {
        myapp.spec.resources = Some(ResourceRequirements {
            cpu: "100m".to_string(),
            memory: "128Mi".to_string(),
        });
    }
    
    // Create JSON patch
    let patch = serde_json::json!([
        {
            "op": "add",
            "path": "/metadata/labels/app.kubernetes.io~1managed-by",
            "value": "myapp-controller"
        }
    ]);
    
    let mut res = AdmissionResponse::from(&req);
    res = res.with_patch(patch).unwrap();
    
    Ok(warp::reply::json(&res.into_review()))
}

// Webhook server
pub async fn run_webhook_server() {
    let validate = warp::post()
        .and(warp::path("validate"))
        .and(warp::body::json())
        .and_then(validate_webhook);
    
    let mutate = warp::post()
        .and(warp::path("mutate"))
        .and(warp::body::json())
        .and_then(mutate_webhook);
    
    let routes = validate.or(mutate);
    
    println!("Starting webhook server on :8443");
    warp::serve(routes)
        .tls()
        .cert_path("tls.crt")
        .key_path("tls.key")
        .run(([0, 0, 0, 0], 8443))
        .await;
}

// ============================================================================
// CONTROLLER with Finalizers and Owner References
// ============================================================================

use kube::runtime::controller::{Action, Controller};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReconcileError {
    #[error("Kube error: {0}")]
    KubeError(#[from] kube::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Finalizer error: {0}")]
    FinalizerError(String),
}

pub struct Context {
    pub client: Client,
}

pub async fn reconcile(myapp: Arc<MyApp>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    let ns = myapp.namespace().unwrap();
    let name = myapp.name_any();
    let api: Api<MyApp> = Api::namespaced(ctx.client.clone(), &ns);
    
    // Handle deletion with finalizer
    if myapp.metadata.deletion_timestamp.is_some() {
        if myapp.finalizers().contains(&FINALIZER.to_string()) {
            // Perform cleanup
            cleanup_resources(&myapp, ctx.client.clone()).await
                .map_err(|e| ReconcileError::FinalizerError(e.to_string()))?;
            
            // Remove finalizer
            remove_finalizer(&myapp, ctx.client.clone()).await?;
            println!("Finalizer removed for MyApp {}/{}", ns, name);
        }
        return Ok(Action::await_change());
    }
    
    // Add finalizer if not present
    if !myapp.finalizers().contains(&FINALIZER.to_string()) {
        add_finalizer(&myapp, ctx.client.clone()).await?;
        println!("Finalizer added for MyApp {}/{}", ns, name);
        return Ok(Action::requeue(std::time::Duration::from_secs(1)));
    }
    
    // Validate the resource
    myapp.validate()
        .map_err(ReconcileError::ValidationError)?;
    
    println!("Reconciling MyApp {}/{}", ns, name);
    
    // Create or update Deployment with owner reference
    let deployments: Api<Deployment> = Api::namespaced(ctx.client.clone(), &ns);
    let deploy_name = format!("{}-deployment", name);
    
    match deployments.get_opt(&deploy_name).await? {
        Some(_) => {
            println!("Deployment {} already exists", deploy_name);
        }
        None => {
            create_deployment(&myapp, ctx.client.clone()).await?;
            println!("Created deployment {} with owner reference", deploy_name);
        }
    }
    
    // Create or update Service with owner reference
    let services: Api<Service> = Api::namespaced(ctx.client.clone(), &ns);
    let svc_name = format!("{}-service", name);
    
    match services.get_opt(&svc_name).await? {
        Some(_) => {
            println!("Service {} already exists", svc_name);
        }
        None => {
            create_service(&myapp, ctx.client.clone()).await?;
            println!("Created service {} with owner reference", svc_name);
        }
    }
    
    // Update status subresource
    let new_status = MyAppStatus {
        state: "Running".to_string(),
        observed_generation: myapp.metadata.generation,
        conditions: vec![
            Condition::ready(true, "ReconcileSuccess", "Resource reconciled successfully")
        ],
        last_updated: Some(chrono::Utc::now().to_rfc3339()),
    };
    
    let status_patch = serde_json::json!({
        "status": new_status
    });
    
    api.patch_status(
        &name,
        &PatchParams::default(),
        &Patch::Merge(&status_patch)
    ).await?;
    
    Ok(Action::requeue(std::time::Duration::from_secs(300)))
}

pub fn error_policy(
    _myapp: Arc<MyApp>,
    error: &ReconcileError,
    _ctx: Arc<Context>,
) -> Action {
    eprintln!("Reconciliation error: {:?}", error);
    Action::requeue(std::time::Duration::from_secs(60))
}

// ============================================================================
// Main - Choose to run controller or webhook server
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "webhook" {
        // Run webhook server
        run_webhook_server().await;
    } else {
        // Run controller
        let client = Client::try_default().await?;
        let context = Arc::new(Context {
            client: client.clone(),
        });
        
        let myapps = Api::<MyApp>::all(client);
        
        Controller::new(myapps, Default::default())
            .run(reconcile, error_policy, context)
            .for_each(|res| async move {
                match res {
                    Ok(o) => println!("Reconciled: {:?}", o),
                    Err(e) => eprintln!("Reconcile error: {:?}", e),
                }
            })
            .await;
    }
    
    Ok(())
}

// Cargo.toml dependencies:
// [dependencies]
// kube = { version = "0.95", features = ["runtime", "derive", "admission"] }
// k8s-openapi = { version = "0.23", features = ["latest"] }
// schemars = "0.8"
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// tokio = { version = "1.0", features = ["full"] }
// thiserror = "1.0"
// chrono = "0.4"
// warp = "0.3"
```


```yaml
# validatingwebhook.yaml
---
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingWebhookConfiguration
metadata:
  name: myapp-validator
webhooks:
  - name: validate.myapps.example.com
    admissionReviewVersions: ["v1", "v1beta1"]
    clientConfig:
      service:
        name: myapp-webhook
        namespace: default
        path: /validate
      caBundle: LS0tLS1CRUdJTi... # Base64 encoded CA cert
    rules:
      - operations: ["CREATE", "UPDATE"]
        apiGroups: ["example.com"]
        apiVersions: ["v1"]
        resources: ["myapps"]
    failurePolicy: Fail
    sideEffects: None
    timeoutSeconds: 10

---
# mutatingwebhook.yaml
apiVersion: admissionregistration.k8s.io/v1
kind: MutatingWebhookConfiguration
metadata:
  name: myapp-mutator
webhooks:
  - name: mutate.myapps.example.com
    admissionReviewVersions: ["v1", "v1beta1"]
    clientConfig:
      service:
        name: myapp-webhook
        namespace: default
        path: /mutate
      caBundle: LS0tLS1CRUdJTi... # Base64 encoded CA cert
    rules:
      - operations: ["CREATE", "UPDATE"]
        apiGroups: ["example.com"]
        apiVersions: ["v1"]
        resources: ["myapps"]
    failurePolicy: Fail
    sideEffects: None
    timeoutSeconds: 10

---
# webhook-service.yaml
apiVersion: v1
kind: Service
metadata:
  name: myapp-webhook
  namespace: default
spec:
  selector:
    app: myapp-webhook
  ports:
    - port: 443
      targetPort: 8443
      protocol: TCP

---
# webhook-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: myapp-webhook
  namespace: default
spec:
  replicas: 2
  selector:
    matchLabels:
      app: myapp-webhook
  template:
    metadata:
      labels:
        app: myapp-webhook
    spec:
      containers:
        - name: webhook
          image: myapp-controller:latest
          command: ["/app/myapp-controller", "webhook"]
          ports:
            - containerPort: 8443
              name: webhook
          volumeMounts:
            - name: webhook-certs
              mountPath: /etc/webhook/certs
              readOnly: true
          env:
            - name: TLS_CERT_FILE
              value: /etc/webhook/certs/tls.crt
            - name: TLS_KEY_FILE
              value: /etc/webhook/certs/tls.key
      volumes:
        - name: webhook-certs
          secret:
            secretName: myapp-webhook-certs

---
# example-myapp.yaml - Example resource with owner references demo
apiVersion: example.com/v1
kind: MyApp
metadata:
  name: sample-app
  namespace: default
  labels:
    environment: production
spec:
  replicas: 3
  image: nginx:1.21.0
  envVars:
    APP_ENV: production
    LOG_LEVEL: info
  resources:
    cpu: "500m"
    memory: "512Mi"
```

```sh
#!/bin/bash
# generate-webhook-certs.sh
# Script to generate TLS certificates for the webhook server

set -e

SERVICE_NAME="myapp-webhook"
NAMESPACE="default"
SECRET_NAME="myapp-webhook-certs"

echo "Generating certificates for webhook..."

# Create a temporary directory
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Generate CA private key
openssl genrsa -out ca.key 2048

# Generate CA certificate
cat > ca.conf <<EOF
[req]
distinguished_name = req_distinguished_name
prompt = no

[req_distinguished_name]
CN = ${SERVICE_NAME}-ca
EOF

openssl req -x509 -new -nodes -key ca.key -days 3650 -out ca.crt -config ca.conf

# Generate server private key
openssl genrsa -out tls.key 2048

# Generate server certificate signing request
cat > server.conf <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = ${SERVICE_NAME}.${NAMESPACE}.svc

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = ${SERVICE_NAME}
DNS.2 = ${SERVICE_NAME}.${NAMESPACE}
DNS.3 = ${SERVICE_NAME}.${NAMESPACE}.svc
DNS.4 = ${SERVICE_NAME}.${NAMESPACE}.svc.cluster.local
EOF

openssl req -new -key tls.key -out server.csr -config server.conf

# Sign the server certificate with CA
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key \
    -CAcreateserial -out tls.crt -days 3650 \
    -extensions v3_req -extfile server.conf

echo "Certificates generated successfully!"

# Create Kubernetes secret
echo "Creating Kubernetes secret..."
kubectl create secret tls ${SECRET_NAME} \
    --cert=tls.crt \
    --key=tls.key \
    --namespace=${NAMESPACE} \
    --dry-run=client -o yaml | kubectl apply -f -

# Get CA bundle for webhook config
CA_BUNDLE=$(cat ca.crt | base64 | tr -d '\n')

echo ""
echo "CA Bundle (use this in your webhook configurations):"
echo "$CA_BUNDLE"

# Update webhook configurations with CA bundle
echo ""
echo "Updating webhook configurations with CA bundle..."

# Update ValidatingWebhookConfiguration
kubectl patch validatingwebhookconfiguration myapp-validator \
    --type='json' \
    -p="[{'op': 'replace', 'path': '/webhooks/0/clientConfig/caBundle', 'value':'${CA_BUNDLE}'}]" 2>/dev/null || echo "ValidatingWebhookConfiguration not found, skipping..."

# Update MutatingWebhookConfiguration
kubectl patch mutatingwebhookconfiguration myapp-mutator \
    --type='json' \
    -p="[{'op': 'replace', 'path': '/webhooks/0/clientConfig/caBundle', 'value':'${CA_BUNDLE}'}]" 2>/dev/null || echo "MutatingWebhookConfiguration not found, skipping..."

# Cleanup
cd -
rm -rf "$TEMP_DIR"

echo ""
echo "Setup complete! The webhook server can now use the certificates from the secret ${SECRET_NAME}"
echo ""
echo "Next steps:"
echo "1. Deploy the webhook server: kubectl apply -f webhook-deployment.yaml"
echo "2. Apply webhook configurations: kubectl apply -f validatingwebhook.yaml mutatingwebhook.yaml"
```

# Kubernetes Custom Resource Controller - Complete Guide

A production-ready Kubernetes operator implementation in Rust demonstrating best practices for CRDs, finalizers, owner references, and webhooks.

## Features

✅ **Custom Resource Definition (CRD)** with status subresource  
✅ **Finalizers** for cleanup before deletion  
✅ **Owner References** for automatic garbage collection  
✅ **Validating Webhooks** for admission control  
✅ **Mutating Webhooks** for default values  
✅ **Status Conditions** following Kubernetes conventions  
...