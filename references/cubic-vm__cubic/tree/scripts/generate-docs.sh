#!/bin/bash
set -euo pipefail

version="$1"

CMDS=(run create instances images ports show modify console ssh scp exec start \
    stop restart rename clone snapshot restore delete prune completions)

function generate_cmd_doc() {
    name="$1"
    ref="$2"
    file="$3"
    shift 3
    underline="${name//?/=}"

    echo -e ".. $ref:\n\n$name\n$underline\n\n.. code-block::\n\n    \$ $name --help" > "$file"
    cargo run -- "$@" --help | sed "s/^/    /" >> "$file"
}

# Set version
sed "s/^release = .*$/release = '$version'/g" -i docs/conf.py

# Create Command Reference Doc Directory
mkdir -p docs/reference/commands

# Generate cubic help
generate_cmd_doc "cubic" "_ref_cubic" "docs/reference/commands/cubic.rst"

# Generate cubic subcommands help
for cmd in "${CMDS[@]}"; do
    generate_cmd_doc "cubic $cmd" "_ref_cubic_$cmd" "docs/reference/commands/$cmd.rst" "$cmd"
done

# Generate index.rst with one toctree per Diataxis section
cat > docs/index.rst << 'EOF'
Cubic
=====

.. toctree::
   :caption: Tutorial
   :hidden:

   tutorial/getting_started
   tutorial/templates
   tutorial/snapshots
   tutorial/isolate

.. toctree::
   :caption: How-to Guides
   :hidden:

   howto/install
   howto/temporary_vm
   howto/exec
   howto/copy_files
   howto/resources
   howto/snapshots
   howto/templates
   howto/ports
   howto/ssh_connect
   howto/console_login
   howto/env_vars
   howto/proxy
   howto/qemu_not_found
   howto/recover_disk

.. toctree::
   :caption: Reference
   :hidden:

   reference/images
   reference/instance
   reference/template
   reference/values
   reference/guest
   reference/files
   reference/environment_variables

.. toctree::
   :caption: Explanation
   :hidden:

   explanation/design
   explanation/machine
   explanation/networking
   explanation/security
   explanation/qemu_detection

.. toctree::
   :caption: Commands
   :hidden:

   reference/commands/cubic
EOF

for cmd in "${CMDS[@]}"; do
    echo "   reference/commands/$cmd" >> docs/index.rst
done

cat >> docs/index.rst << 'EOF'

Cubic spins up Linux virtual machines on Linux, macOS and Windows with a single command.

Every distribution comes as an official image and is ready to use within seconds, so you skip the long installation. Cubic keeps things simple and secure by acting as lightweight glue over proven tools. No privileged system service is required and every VM runs as your normal user.
Cubic is built on top of ``QEMU``, ``EDK2``, official Linux distribution images and ``cloud-init``.

The Tutorial builds your first virtual machine step by step.
The How-to Guides each solve one task.
The Reference lists the settings.
The Explanation covers how Cubic works and why.
The Commands pages list every command and its options.

Features
---------

**Fast and simple**

* Creates a VM and opens a shell in one command
* Boots official Linux distribution images in seconds
* Written in Rust

**Runs anywhere**

* Runs on **Linux**, **macOS** and **Windows** hosts
* Ships **Alma Linux**, **Arch Linux**, **Debian**, **Fedora**, **Gentoo**, **OpenSUSE**, **Rocky Linux** and **Ubuntu**
* Runs **amd64** and **arm64** guests
* Uses the hardware accelerator of the host, **KVM** on Linux, **Hypervisor** on macOS, **WHPX** on Windows and **NVMM** on BSD

**Everyday work**

* Forwards ports from a VM to the host
* Copies files between host and VM and between two VMs
* Executes single commands in a VM
* Creates VM instances from reusable templates
* Snapshots a VM disk and restores it later
* Clones and renames VM instances
* Runs temporary VM instances that are deleted after use
* Isolates a VM from the network with one flag

**Safe by default**

* Runs every VM as a normal user process without a privileged system service
* Verifies every image against the checksum of the distribution
* Protects every VM with its own SSH key and a locked password
* Encrypts the QEMU control channels with mutual TLS

Source Code
-----------

The source code of Cubic is on `Github`_.

.. _Github: https://github.com/cubic-vm/cubic
EOF
