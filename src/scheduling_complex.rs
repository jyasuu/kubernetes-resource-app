// Advanced scheduling module for MyApp Controller
// Provides sophisticated scheduling capabilities including node affinity,
// pod anti-affinity, resource quotas, and custom placement strategies

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::collections::BTreeMap;

/// Advanced scheduling configuration for MyApp resources
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SchedulingConfig {
    /// Node selection preferences
    #[serde(default)]
    pub node_selector: BTreeMap<String, String>,
    
    /// Node affinity rules
    #[serde(default)]
    pub node_affinity: Option<NodeAffinityConfig>,
    
    /// Pod affinity rules
    #[serde(default)]
    pub pod_affinity: Option<PodAffinityConfig>,
    
    /// Pod anti-affinity rules
    #[serde(default)]
    pub pod_anti_affinity: Option<PodAntiAffinityConfig>,
    
    /// Tolerations for node taints
    #[serde(default)]
    pub tolerations: Vec<TolerationConfig>,
    
    /// Topology spread constraints
    #[serde(default)]
    pub topology_spread_constraints: Vec<TopologySpreadConfig>,
    
    /// Priority class for pod scheduling
    #[serde(default)]
    pub priority_class: Option<String>,
    
    /// Scheduler name (for custom schedulers)
    #[serde(default)]
    pub scheduler_name: Option<String>,
    
    /// Resource quotas and limits
    #[serde(default)]
    pub resource_policy: Option<ResourcePolicy>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeAffinityConfig {
    /// Required node affinity (hard constraint)
    #[serde(default)]
    pub required: Vec<NodeSelectorConfig>,
    
    /// Preferred node affinity (soft constraint with weights)
    #[serde(default)]
    pub preferred: Vec<PreferredNodeSelectorConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectorConfig {
    /// Label key to match
    pub key: String,
    
    /// Operator (In, NotIn, Exists, DoesNotExist, Gt, Lt)
    pub operator: String,
    
    /// Values to match (optional for Exists/DoesNotExist)
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreferredNodeSelectorConfig {
    /// Weight for this preference (1-100)
    #[schemars(range(min = 1, max = 100))]
    pub weight: i32,
    
    /// Node selector terms
    pub selector: NodeSelectorConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PodAffinityConfig {
    /// Required pod affinity (hard constraint)
    #[serde(default)]
    pub required: Vec<PodAffinityTermConfig>,
    
    /// Preferred pod affinity (soft constraint with weights)
    #[serde(default)]
    pub preferred: Vec<WeightedPodAffinityTermConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PodAntiAffinityConfig {
    /// Required pod anti-affinity (hard constraint)
    #[serde(default)]
    pub required: Vec<PodAffinityTermConfig>,
    
    /// Preferred pod anti-affinity (soft constraint with weights)
    #[serde(default)]
    pub preferred: Vec<WeightedPodAffinityTermConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PodAffinityTermConfig {
    /// Label selector for matching pods
    pub label_selector: BTreeMap<String, String>,
    
    /// Topology key (e.g., "kubernetes.io/hostname", "topology.kubernetes.io/zone")
    pub topology_key: String,
    
    /// Namespaces to consider (empty means same namespace)
    #[serde(default)]
    pub namespaces: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WeightedPodAffinityTermConfig {
    /// Weight for this affinity term (1-100)
    #[schemars(range(min = 1, max = 100))]
    pub weight: i32,
    
    /// Pod affinity term
    pub pod_affinity_term: PodAffinityTermConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TolerationConfig {
    /// Taint key to tolerate
    pub key: String,
    
    /// Operator (Equal, Exists)
    pub operator: String,
    
    /// Taint value (required for Equal operator)
    #[serde(default)]
    pub value: Option<String>,
    
    /// Effect (NoSchedule, PreferNoSchedule, NoExecute)
    pub effect: String,
    
    /// Toleration seconds (for NoExecute effect)
    #[serde(default)]
    pub toleration_seconds: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TopologySpreadConfig {
    /// Maximum allowed difference between any two topology domains
    pub max_skew: i32,
    
    /// Topology key to spread across
    pub topology_key: String,
    
    /// How to handle pods that don't match topology spread constraints
    pub when_unsatisfiable: String, // DoNotSchedule or ScheduleAnyway
    
    /// Label selector for pods to consider
    pub label_selector: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePolicy {
    /// Default resource requests and limits
    #[serde(default)]
    pub defaults: Option<ResourceLimits>,
    
    /// Minimum allowed resources
    #[serde(default)]
    pub min: Option<ResourceLimits>,
    
    /// Maximum allowed resources
    #[serde(default)]
    pub max: Option<ResourceLimits>,
    
    /// Resource scaling policy
    #[serde(default)]
    pub scaling: Option<ScalingPolicy>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLimits {
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub storage: Option<String>,
    
    /// Custom resources (e.g., GPUs)
    #[serde(default)]
    pub custom: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScalingPolicy {
    /// Enable horizontal pod autoscaling
    pub enable_hpa: bool,
    
    /// Target CPU utilization percentage
    #[serde(default)]
    pub target_cpu_utilization: Option<i32>,
    
    /// Target memory utilization percentage
    #[serde(default)]
    pub target_memory_utilization: Option<i32>,
    
    /// Minimum replicas for HPA
    #[serde(default)]
    pub min_replicas: Option<i32>,
    
    /// Maximum replicas for HPA
    #[serde(default)]
    pub max_replicas: Option<i32>,
    
    /// Custom metrics for scaling
    #[serde(default)]
    pub custom_metrics: Vec<CustomMetric>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomMetric {
    pub name: String,
    pub target_value: String,
    pub metric_type: String, // Pods, Object, External
}

/// Scheduler implementation for advanced placement strategies
pub struct AdvancedScheduler;

impl AdvancedScheduler {
    /// Generate intelligent placement recommendations
    pub fn recommend_placement(
        _app_name: &str,
        _namespace: &str,
        replicas: i32,
        _existing_apps: &[String]
    ) -> SchedulingConfig {
        SchedulingConfig {
            node_selector: BTreeMap::new(),
            node_affinity: None,
            pod_affinity: None,
            pod_anti_affinity: if replicas > 1 {
                Some(PodAntiAffinityConfig {
                    required: vec![],
                    preferred: vec![WeightedPodAffinityTermConfig {
                        weight: 100,
                        pod_affinity_term: PodAffinityTermConfig {
                            label_selector: {
                                let mut selector = BTreeMap::new();
                                selector.insert("app".to_string(), _app_name.to_string());
                                selector
                            },
                            topology_key: "kubernetes.io/hostname".to_string(),
                            namespaces: vec![],
                        },
                    }],
                })
            } else {
                None
            },
            tolerations: Vec::new(),
            topology_spread_constraints: Vec::new(),
            priority_class: None,
            scheduler_name: None,
            resource_policy: Some(ResourcePolicy {
                defaults: Some(ResourceLimits {
                    cpu: Some("100m".to_string()),
                    memory: Some("128Mi".to_string()),
                    storage: None,
                    custom: BTreeMap::new(),
                }),
                min: None,
                max: None,
                scaling: None,
            }),
        }
    }
}
    /// Convert scheduling config to Kubernetes node affinity
    pub fn build_node_affinity(config: &NodeAffinityConfig) -> NodeAffinity {
        let mut node_affinity = NodeAffinity::default();
        
        // Required node affinity
        if !config.required.is_empty() {
            let mut terms = Vec::new();
            for req in &config.required {
                let term = NodeSelectorTerm {
                    match_expressions: Some(vec![NodeSelectorRequirement {
                        key: req.key.clone(),
                        operator: req.operator.clone(),
                        values: if req.values.is_empty() { None } else { Some(req.values.clone()) },
                    }]),
                    match_fields: None,
                };
                terms.push(term);
            }
            
            node_affinity.required_during_scheduling_ignored_during_execution = Some(NodeSelector {
                node_selector_terms: terms,
            });
        }
        
        // Preferred node affinity
        if !config.preferred.is_empty() {
            let mut preferred_terms = Vec::new();
            for pref in &config.preferred {
                let term = k8s_openapi::api::core::v1::PreferredSchedulingTerm {
                    weight: pref.weight,
                    preference: NodeSelectorTerm {
                        match_expressions: Some(vec![NodeSelectorRequirement {
                            key: pref.selector.key.clone(),
                            operator: pref.selector.operator.clone(),
                            values: if pref.selector.values.is_empty() { 
                                None 
                            } else { 
                                Some(pref.selector.values.clone()) 
                            },
                        }]),
                        match_fields: None,
                    },
                };
                preferred_terms.push(term);
            }
            
            node_affinity.preferred_during_scheduling_ignored_during_execution = Some(preferred_terms);
        }
        
        node_affinity
    }
    
    /// Convert scheduling config to Kubernetes pod affinity
    pub fn build_pod_affinity(config: &PodAffinityConfig) -> PodAffinity {
        let mut pod_affinity = PodAffinity::default();
        
        // Required pod affinity
        if !config.required.is_empty() {
            let mut terms = Vec::new();
            for req in &config.required {
                let term = PodAffinityTerm {
                    label_selector: Some(LabelSelector {
                        match_labels: Some(req.label_selector.clone()),
                        match_expressions: None,
                    }),
                    topology_key: req.topology_key.clone(),
                    namespaces: if req.namespaces.is_empty() { 
                        None 
                    } else { 
                        Some(req.namespaces.clone()) 
                    },
                    namespace_selector: None,
                };
                terms.push(term);
            }
            pod_affinity.required_during_scheduling_ignored_during_execution = Some(terms);
        }
        
        // Preferred pod affinity
        if !config.preferred.is_empty() {
            let mut weighted_terms = Vec::new();
            for pref in &config.preferred {
                let term = WeightedPodAffinityTerm {
                    weight: pref.weight,
                    pod_affinity_term: PodAffinityTerm {
                        label_selector: Some(LabelSelector {
                            match_labels: Some(pref.pod_affinity_term.label_selector.clone()),
                            match_expressions: None,
                        }),
                        topology_key: pref.pod_affinity_term.topology_key.clone(),
                        namespaces: if pref.pod_affinity_term.namespaces.is_empty() { 
                            None 
                        } else { 
                            Some(pref.pod_affinity_term.namespaces.clone()) 
                        },
                        namespace_selector: None,
                    },
                };
                weighted_terms.push(term);
            }
            pod_affinity.preferred_during_scheduling_ignored_during_execution = Some(weighted_terms);
        }
        
        pod_affinity
    }
    
    /// Convert scheduling config to Kubernetes pod anti-affinity
    pub fn build_pod_anti_affinity(config: &PodAntiAffinityConfig) -> PodAntiAffinity {
        let mut pod_anti_affinity = PodAntiAffinity::default();
        
        // Required pod anti-affinity
        if !config.required.is_empty() {
            let mut terms = Vec::new();
            for req in &config.required {
                let term = PodAffinityTerm {
                    label_selector: Some(LabelSelector {
                        match_labels: Some(req.label_selector.clone()),
                        match_expressions: None,
                    }),
                    topology_key: req.topology_key.clone(),
                    namespaces: if req.namespaces.is_empty() { 
                        None 
                    } else { 
                        Some(req.namespaces.clone()) 
                    },
                    namespace_selector: None,
                };
                terms.push(term);
            }
            pod_anti_affinity.required_during_scheduling_ignored_during_execution = Some(terms);
        }
        
        // Preferred pod anti-affinity
        if !config.preferred.is_empty() {
            let mut weighted_terms = Vec::new();
            for pref in &config.preferred {
                let term = WeightedPodAffinityTerm {
                    weight: pref.weight,
                    pod_affinity_term: PodAffinityTerm {
                        label_selector: Some(LabelSelector {
                            match_labels: Some(pref.pod_affinity_term.label_selector.clone()),
                            match_expressions: None,
                        }),
                        topology_key: pref.pod_affinity_term.topology_key.clone(),
                        namespaces: if pref.pod_affinity_term.namespaces.is_empty() { 
                            None 
                        } else { 
                            Some(pref.pod_affinity_term.namespaces.clone()) 
                        },
                        namespace_selector: None,
                    },
                };
                weighted_terms.push(term);
            }
            pod_anti_affinity.preferred_during_scheduling_ignored_during_execution = Some(weighted_terms);
        }
        
        pod_anti_affinity
    }
    
    /// Convert toleration config to Kubernetes tolerations
    pub fn build_tolerations(configs: &[TolerationConfig]) -> Vec<Toleration> {
        configs.iter().map(|config| {
            Toleration {
                key: Some(config.key.clone()),
                operator: Some(config.operator.clone()),
                value: config.value.clone(),
                effect: Some(config.effect.clone()),
                toleration_seconds: config.toleration_seconds,
            }
        }).collect()
    }
    
    /// Convert topology spread config to Kubernetes constraints
    pub fn build_topology_spread_constraints(configs: &[TopologySpreadConfig]) -> Vec<TopologySpreadConstraint> {
        configs.iter().map(|config| {
            TopologySpreadConstraint {
                max_skew: config.max_skew,
                topology_key: config.topology_key.clone(),
                when_unsatisfiable: config.when_unsatisfiable.clone(),
                label_selector: Some(LabelSelector {
                    match_labels: Some(config.label_selector.clone()),
                    match_expressions: None,
                }),
                min_domains: None,
                node_affinity_policy: None,
                node_taints_policy: None,
            }
        }).collect()
    }
    
    /// Apply resource policy to container resources
    pub fn apply_resource_policy(
        base_resources: &Option<crate::ResourceRequirements>,
        policy: &ResourcePolicy
    ) -> Option<K8sResourceRequirements> {
        let mut requests = BTreeMap::new();
        let mut limits = BTreeMap::new();
        
        // Start with base resources
        if let Some(base) = base_resources {
            requests.insert("cpu".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(base.cpu.clone()));
            requests.insert("memory".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(base.memory.clone()));
            limits.insert("cpu".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(base.cpu.clone()));
            limits.insert("memory".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(base.memory.clone()));
        }
        
        // Apply defaults if no base resources
        if let Some(defaults) = &policy.defaults {
            if requests.is_empty() {
                if let Some(cpu) = &defaults.cpu {
                    requests.insert("cpu".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(cpu.clone()));
                }
                if let Some(memory) = &defaults.memory {
                    requests.insert("memory".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(memory.clone()));
                }
            }
            if limits.is_empty() {
                if let Some(cpu) = &defaults.cpu {
                    limits.insert("cpu".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(cpu.clone()));
                }
                if let Some(memory) = &defaults.memory {
                    limits.insert("memory".to_string(), k8s_openapi::apimachinery::pkg::api::resource::Quantity(memory.clone()));
                }
            }
        }
        
        // TODO: Apply min/max constraints
        // This would require parsing and comparing resource quantities
        
        if requests.is_empty() && limits.is_empty() {
            None
        } else {
            Some(K8sResourceRequirements {
                requests: if requests.is_empty() { None } else { Some(requests) },
                limits: if limits.is_empty() { None } else { Some(limits) },
            })
        }
    }
    
    /// Generate intelligent placement recommendations
    pub fn recommend_placement(
        app_name: &str,
        namespace: &str,
        replicas: i32,
        existing_apps: &[String]
    ) -> SchedulingConfig {
        let mut config = SchedulingConfig {
            node_selector: BTreeMap::new(),
            node_affinity: None,
            pod_affinity: None,
            pod_anti_affinity: None,
            tolerations: Vec::new(),
            topology_spread_constraints: Vec::new(),
            priority_class: None,
            scheduler_name: None,
            resource_policy: None,
        };
        
        // Recommend anti-affinity for multiple replicas
        if replicas > 1 {
            config.pod_anti_affinity = Some(PodAntiAffinityConfig {
                required: vec![],
                preferred: vec![WeightedPodAffinityTermConfig {
                    weight: 100,
                    pod_affinity_term: PodAffinityTermConfig {
                        label_selector: {
                            let mut selector = BTreeMap::new();
                            selector.insert("app".to_string(), app_name.to_string());
                            selector
                        },
                        topology_key: "kubernetes.io/hostname".to_string(),
                        namespaces: vec![],
                    },
                }],
            });
        }
        
        // Recommend topology spread for high replica counts
        if replicas >= 3 {
            config.topology_spread_constraints = vec![
                TopologySpreadConfig {
                    max_skew: 1,
                    topology_key: "topology.kubernetes.io/zone".to_string(),
                    when_unsatisfiable: "DoNotSchedule".to_string(),
                    label_selector: {
                        let mut selector = BTreeMap::new();
                        selector.insert("app".to_string(), app_name.to_string());
                        selector
                    },
                }
            ];
        }
        
        // Add resource policy recommendations
        config.resource_policy = Some(ResourcePolicy {
            defaults: Some(ResourceLimits {
                cpu: Some("100m".to_string()),
                memory: Some("128Mi".to_string()),
                storage: None,
                custom: BTreeMap::new(),
            }),
            min: Some(ResourceLimits {
                cpu: Some("50m".to_string()),
                memory: Some("64Mi".to_string()),
                storage: None,
                custom: BTreeMap::new(),
            }),
            max: Some(ResourceLimits {
                cpu: Some("2".to_string()),
                memory: Some("4Gi".to_string()),
                storage: None,
                custom: BTreeMap::new(),
            }),
            scaling: Some(ScalingPolicy {
                enable_hpa: replicas > 2,
                target_cpu_utilization: Some(70),
                target_memory_utilization: Some(80),
                min_replicas: Some(replicas),
                max_replicas: Some(replicas * 3),
                custom_metrics: Vec::new(),
            }),
        });
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_affinity_conversion() {
        let config = NodeAffinityConfig {
            required: vec![NodeSelectorConfig {
                key: "node-type".to_string(),
                operator: "In".to_string(),
                values: vec!["compute".to_string()],
            }],
            preferred: vec![PreferredNodeSelectorConfig {
                weight: 50,
                selector: NodeSelectorConfig {
                    key: "instance-type".to_string(),
                    operator: "In".to_string(),
                    values: vec!["m5.large".to_string()],
                },
            }],
        };
        
        let affinity = AdvancedScheduler::build_node_affinity(&config);
        assert!(affinity.required_during_scheduling_ignored_during_execution.is_some());
        assert!(affinity.preferred_during_scheduling_ignored_during_execution.is_some());
    }
    
    #[test]
    fn test_placement_recommendations() {
        let config = AdvancedScheduler::recommend_placement(
            "test-app",
            "default",
            5,
            &[]
        );
        
        assert!(config.pod_anti_affinity.is_some());
        assert!(!config.topology_spread_constraints.is_empty());
        assert!(config.resource_policy.is_some());
    }
    
    #[test]
    fn test_toleration_conversion() {
        let configs = vec![TolerationConfig {
            key: "node.kubernetes.io/not-ready".to_string(),
            operator: "Exists".to_string(),
            value: None,
            effect: "NoExecute".to_string(),
            toleration_seconds: Some(300),
        }];
        
        let tolerations = AdvancedScheduler::build_tolerations(&configs);
        assert_eq!(tolerations.len(), 1);
        assert_eq!(tolerations[0].toleration_seconds, Some(300));
    }
}