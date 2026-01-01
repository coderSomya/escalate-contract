#!/bin/bash

cargo b --release --target wasm32-unknown-unknown

cp $(pwd)/target/wasm32-unknown-unknown/release/escalate.wasm $(pwd)/build/escalate.wasm
cp $(pwd)/escalate.widl                                       $(pwd)/build/escalate.widl