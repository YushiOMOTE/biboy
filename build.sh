#!/bin/sh

set -xe

project=biboy

mode=${mode:-release}

# Kernel settings
kernel_dir=$(pwd)
kernel_target=$kernel_dir/target/x86_64-$project/$mode
kernel_binary=$kernel_target/$project
kernel_manifest=$kernel_dir/Cargo.toml

# Bootloader settings
loader_dir=$(pwd)/ext/bootloader
loader_target=$loader_dir/target/x86_64-bootloader/$mode
loader_binary=$loader_target/bootloader
loader_bootable=$loader_target/bootloader.bin

flag=$([ $mode == "release" ] && echo "--release" || echo "")

case $1 in
    setup)
        git submodule update --init --recursive
        rustup component add llvm-tools-preview
        cargo install cargo-binutils
        ;;
    build)
	      # Load config
	      source vars.sh

        # Build kernel
        cargo xbuild $flag

        # Build bootloader with kernel as its payload
        pushd $loader_dir
        export KERNEL=$kernel_binary
        export KERNEL_MANIFEST=$kernel_manifest
        cargo xbuild $flag --features binary,vga_320x200,map_physical_memory,recursive_page_table
        cargo objcopy -- --strip-all -I elf64-x86-64 -O binary \
              --binary-architecture=i386:x86-64 \
              $loader_binary $loader_bootable
        popd

        ;;
    run)
        $0 build
        qemu-system-x86_64 -m 4G -serial stdio -drive format=raw,file=$loader_bootable
        ;;
    clean)
        cargo clean

        pushd $loader_dir
        cargo clean
        popd
        ;;
    help)
        echo "usage: $0 setup|build|run|clean"
        ;;
    *)
        $0 build
esac
