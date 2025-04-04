#!/bin/bash
# This script builds the SoX library for both x86_64 and arm64 architectures on macOS.
# Get the source code
git clone https://github.com/chirlu/sox.git
cd sox

# Configure with both architectures
./autogen.sh
CFLAGS="-arch x86_64 -arch arm64" LDFLAGS="-arch x86_64 -arch arm64" ./configure

# Build and install
make
sudo make install