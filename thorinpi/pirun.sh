#/usr/bin/env sh
cd "$(dirname "$0")"

export PKG_CONFIG_ALLOW_CROSS=1
#export PKG_CONFIG_ALL_STATIC=1
#export OPENSSL_STATIC=1
export OPENSSL_LIB_DIR=/home/rumatoest/workspace/openssl-1.1.0f/
export OPENSSL_INCLUDE_DIR=/home/rumatoest/workspace/openssl-1.1.0f/include

# cargo build --release --target=arm-unknown-linux-gnueabihf
cargo build --target=arm-unknown-linux-gnueabihf

# spawn ssh MyUserName@192.168.20.20
# expect "password"
# send "MyPassword\r"
# interact

FILE="./target/arm-unknown-linux-gnueabihf/debug/ThorinPi"
# FILE="./target/arm-unknown-linux-gnueabihf/release/ThorinPi"
echo "SCP $FILE"
sshpass -v -p raspberry  scp $FILE pi@192.168.10.60:~/bin
# sshpass -v -p raspberry  scp ./target/arm-unknown-linux-gnueabihf/release/ThorinPi pi@192.168.10.60:~/bin