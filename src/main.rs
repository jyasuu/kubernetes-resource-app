use kube::{CustomResource, CustomResourceExt, Resource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use futures_util::StreamExt;
use json_patch::{Patch as JsonPatch, PatchOperation, AddOperation};

mod metrics;
mod scheduling;

use metrics::{MetricsCollector, metrics_handler, health_handler, ready_handler};
use scheduling::{SchedulingConfig, AdvancedScheduler};

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
    
    /// Advanced scheduling configuration
    #[serde(default)]
    pub scheduling: Option<SchedulingConfig>,
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
    
    let myapp = match &req.object {
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
    
    let myapp = match &req.object {
        Some(obj) => obj,
        None => {
            return Ok(warp::reply::json(&AdmissionResponse::invalid(
                "No object in request".to_string()
            ).into_review()));
        }
    };
    
    // Create JSON patch to add default labels and resources
    let mut patches = Vec::new();
    
    // Add managed-by label
    patches.push(PatchOperation::Add(AddOperation {
        path: "/metadata/labels/app.kubernetes.io~1managed-by".parse().unwrap(),
        value: serde_json::Value::String("myapp-controller".to_string()),
    }));
    
    // Add default resources if not specified
    if myapp.spec.resources.is_none() {
        patches.push(PatchOperation::Add(AddOperation {
            path: "/spec/resources".parse().unwrap(),
            value: serde_json::json!({
                "cpu": "100m",
                "memory": "128Mi"
            }),
        }));
    }
    
    let patch = JsonPatch(patches);
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
    pub metrics: MetricsCollector,
}

pub async fn reconcile(myapp: Arc<MyApp>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    let ns = myapp.namespace().unwrap();
    let name = myapp.name_any();
    let api: Api<MyApp> = Api::namespaced(ctx.client.clone(), &ns);
    
    // Start metrics timer
    let timer = ctx.metrics.start_reconcile(&ns, &name);
    
    // Handle deletion with finalizer
    if myapp.metadata.deletion_timestamp.is_some() {
        if myapp.finalizers().contains(&FINALIZER.to_string()) {
            // Perform cleanup
            cleanup_resources(&myapp, ctx.client.clone()).await
                .map_err(|e| {
                    ctx.metrics.record_error("finalizer_cleanup_error", &ns);
                    ReconcileError::FinalizerError(e.to_string())
                })?;
            
            // Remove finalizer
            remove_finalizer(&myapp, ctx.client.clone()).await.map_err(|e| {
                ctx.metrics.record_error("finalizer_removal_error", &ns);
                e
            })?;
            println!("Finalizer removed for MyApp {}/{}", ns, name);
        }
        timer.success();
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
        .map_err(|e| {
            ctx.metrics.record_error("validation_error", &ns);
            ReconcileError::ValidationError(e)
        })?;
    
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
    
    // Update metrics
    ctx.metrics.set_managed_resources("deployment", &ns, 1);
    ctx.metrics.set_managed_resources("service", &ns, 1);
    
    timer.success();
    Ok(Action::requeue(std::time::Duration::from_secs(300)))
}

pub fn error_policy(
    myapp: Arc<MyApp>,
    error: &ReconcileError,
    ctx: Arc<Context>,
) -> Action {
    let ns = myapp.namespace().unwrap_or_default();
    
    // Record error in metrics
    let error_type = match error {
        ReconcileError::KubeError(_) => "kube_error",
        ReconcileError::ValidationError(_) => "validation_error",
        ReconcileError::FinalizerError(_) => "finalizer_error",
    };
    ctx.metrics.record_error(error_type, &ns);
    
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
    } else if args.len() > 1 && args[1] == "generate-crd" {
        // Generate CRD YAML
        let crd = MyApp::crd();
        let yaml = serde_yaml::to_string(&crd)?;
        
        std::fs::write("crd.yaml", yaml)?;
        println!("CRD written to crd.yaml");
    } else {
        // Run controller
        let client = Client::try_default().await?;
        let metrics = MetricsCollector::new();
        let context = Arc::new(Context {
            client: client.clone(),
            metrics,
        });
        
        let myapps = Api::<MyApp>::all(client);
        
        // Start metrics server
        let metrics_routes = metrics_handler()
            .or(health_handler())
            .or(ready_handler());
        
        tokio::spawn(async {
            println!("Starting metrics server on :8080");
            warp::serve(metrics_routes)
                .run(([0, 0, 0, 0], 8080))
                .await;
        });
        
        // Start health server
        tokio::spawn(async {
            let health_routes = health_handler().or(ready_handler());
            println!("Starting health server on :8081");
            warp::serve(health_routes)
                .run(([0, 0, 0, 0], 8081))
                .await;
        });
        
        println!("Starting MyApp controller...");
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
