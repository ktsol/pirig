SEE https://github.com/sfackler/rust-openssl

You must pass shared option when configuring openssl compilation (this will make -fPIC parameter be passed to the compiler).

Here is a sequence of commands that I used to test cross compiling a Rust program that prints the openssl version:

cd /tmp

wget https://www.openssl.org/source/openssl-1.0.1t.tar.gz
tar xzf openssl-1.0.1t.tar.gz
export MACHINE=armv7
export ARCH=arm
export CC=arm-linux-gnueabihf-gcc
cd openssl-1.0.1t && ./config shared && make && cd -

export OPENSSL_LIB_DIR=/tmp/openssl-1.0.1t/
export OPENSSL_INCLUDE_DIR=/tmp/openssl-1.0.1t/include
cargo new xx --bin
cd xx
mkdir .cargo
cat > .cargo/config << EOF
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
EOF

cat > src/main.rs << EOF
extern crate openssl;

fn main() {
    println!("{}", openssl::version::version())
}
EOF

cargo add openssl # requires cargo install cargo-add
cargo build --target armv7-unknown-linux-gnueabihf