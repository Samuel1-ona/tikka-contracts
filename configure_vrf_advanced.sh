#!/bin/bash

# Advanced VRF configuration using raw data
# This attempts to use your large subscription ID

echo "Attempting to configure VRF with large subscription ID..."

# Your large subscription ID
SUBSCRIPTION_ID="104463245950711925848002979325066164966376483155872594786291361268349330657388"

# VRF parameters
KEY_HASH="0x9e1344a1247c8a1785d0a4681a27152bffdb43666ae5bf7d14d24a5efd44bf71"
CALLBACK_GAS_LIMIT="200000"
REQUEST_CONFIRMATIONS="3"

# Contract address
CONTRACT_ADDRESS="0xed32402c968d04D1d7F6B3DEfcB7A91321736156"
RPC_URL="https://base-sepolia.infura.io/v3/2DmS9CrnVeU2Caun612yGaPQ2aq"

echo "WARNING: This may fail due to uint64 overflow!"
echo "Subscription ID: $SUBSCRIPTION_ID"

# Try to call configureVRF (this will likely fail)
cast send $CONTRACT_ADDRESS \
    "configureVRF(uint64,bytes32,uint32,uint16)" \
    $SUBSCRIPTION_ID \
    $KEY_HASH \
    $CALLBACK_GAS_LIMIT \
    $REQUEST_CONFIRMATIONS \
    --rpc-url $RPC_URL \
    --private-key $PRIVATE_KEY

echo "If this failed, you need to create a new subscription with a smaller ID."
