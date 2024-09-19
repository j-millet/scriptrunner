#!/bin/bash
git clone https://github.com/j-millet/scriptrunner.git scriptrunner-src
cd scriptrunner-src
echo 'building'
cargo build -r
cd ..

echo 'copying binary to /usr/bin'
sudo cp scriptrunner-src/target/release/scriptrunner /usr/bin/scriptrunner

echo 'creating config file in ~/.config/scriptrunner'
mkdir ~/.config/scriptrunner
touch ~/.config/scriptrunner/config

echo 'removing source code'
rm -fr scriptrunner-src