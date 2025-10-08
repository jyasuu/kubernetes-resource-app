#!/bin/bash
# Deployment script for MyApp Kubernetes Controller

set -e

echo "🚀 Deploying MyApp Kubernetes Controller"

# Step 1: Generate and apply CRD
echo "📋 Generating and applying CRD..."
cargo run -- generate-crd
kubectl apply -f crd.yaml
echo "✅ CRD applied successfully"

# Step 2: Create RBAC
echo "🔐 Creating RBAC..."
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
echo "✅ RBAC created successfully"

# Step 3: Build Docker image
echo "🐳 Building Docker image..."
docker build -t myapp-controller:latest .
echo "✅ Docker image built successfully"

# Step 4: Deploy controller
echo "🚢 Deploying controller..."
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
        image: myapp-controller:latest
        imagePullPolicy: Never
        env:
        - name: RUST_LOG
          value: info
EOF
echo "✅ Controller deployed successfully"

# Step 5: Test with sample resource
echo "🧪 Creating sample MyApp resource..."
kubectl apply -f examples/sample-myapp.yaml
echo "✅ Sample resource created"

echo ""
echo "🎉 Deployment completed successfully!"
echo ""
echo "To check the status:"
echo "  kubectl get myapps"
echo "  kubectl describe myapp sample-app"
echo "  kubectl logs -f deployment/myapp-controller"
echo ""
echo "To test cleanup:"
echo "  kubectl delete myapp sample-app"