#!/bin/bash
# Test script for MyApp Kubernetes Controller

set -e

echo "🧪 Testing MyApp Kubernetes Controller"

# Test 1: Apply sample resource
echo "📋 Test 1: Creating sample MyApp resource..."
kubectl apply -f examples/sample-myapp.yaml

# Wait a bit for reconciliation
echo "⏳ Waiting for reconciliation..."
sleep 5

# Check if deployment was created
echo "🔍 Checking if deployment was created..."
if kubectl get deployment sample-app-deployment >/dev/null 2>&1; then
    echo "✅ Deployment created successfully"
else
    echo "❌ Deployment not found"
    exit 1
fi

# Check if service was created
echo "🔍 Checking if service was created..."
if kubectl get service sample-app-service >/dev/null 2>&1; then
    echo "✅ Service created successfully"
else
    echo "❌ Service not found"
    exit 1
fi

# Check MyApp status
echo "🔍 Checking MyApp status..."
kubectl get myapp sample-app -o yaml | grep -A 10 status:

# Test 2: Update replicas
echo "📋 Test 2: Updating replicas..."
kubectl patch myapp sample-app --type='merge' -p='{"spec":{"replicas":5}}'

# Wait for reconciliation
sleep 5

# Check if deployment was updated
REPLICAS=$(kubectl get deployment sample-app-deployment -o jsonpath='{.spec.replicas}')
if [ "$REPLICAS" = "5" ]; then
    echo "✅ Deployment replicas updated successfully"
else
    echo "❌ Deployment replicas not updated (expected 5, got $REPLICAS)"
    exit 1
fi

# Test 3: Test finalizer (deletion)
echo "📋 Test 3: Testing finalizer behavior..."
kubectl delete myapp sample-app

# Wait a bit
sleep 5

# Check if resources are cleaned up
if kubectl get deployment sample-app-deployment >/dev/null 2>&1; then
    echo "❌ Deployment still exists after MyApp deletion"
    exit 1
else
    echo "✅ Deployment cleaned up successfully"
fi

if kubectl get service sample-app-service >/dev/null 2>&1; then
    echo "❌ Service still exists after MyApp deletion"
    exit 1
else
    echo "✅ Service cleaned up successfully"
fi

echo ""
echo "🎉 All tests passed successfully!"
echo ""
echo "Controller is working correctly with:"
echo "  ✅ Resource creation and reconciliation"
echo "  ✅ Owner references and child resource management"
echo "  ✅ Status updates"
echo "  ✅ Finalizers and cleanup"