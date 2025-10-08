#!/bin/bash
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
echo "1. Deploy the webhook server: kubectl apply -f deploy/webhook-manifests.yaml"
echo "2. Apply webhook configurations if not already applied"