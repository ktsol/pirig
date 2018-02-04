#/usr/bin/env sh

cargo build --target=arm-unknown-linux-gnueabihf

# spawn ssh MyUserName@192.168.20.20
# expect "password"
# send "MyPassword\r"
# interact

sshpass -v -p raspberry  scp ./target/arm-unknown-linux-gnueabihf/debug/ThorinPi pi@192.168.11.11:~