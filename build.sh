#!/bin/bash

cd dummy-parachain
cargo build
cd -

cd spree-lamport-clock
cargo build
cd -

cd polkadot-re-mock
cargo build
cd -
