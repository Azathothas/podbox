.. _Install Cubic:

Install Cubic
=============

Ubuntu (Snap)
-------------

.. code-block::

    sudo snap install cubic && \
    sudo snap connect cubic:kvm

The second command connects the KVM interface, which lets VM instances
use hardware acceleration. The package is on the `Snap Store`_.

.. _Snap Store: https://snapcraft.io/cubic

macOS (Homebrew)
----------------

.. code-block::

    brew install cubic-vm/cubic/cubic

The formula lives in the `Homebrew tap`_ and also installs on Linux.

.. _Homebrew tap: https://github.com/cubic-vm/homebrew-cubic

Windows (winget)
----------------

.. code-block::

    winget install cubic-vm.cubic

QEMU is installed automatically as a dependency of the `winget package`_.

.. _winget package: https://learn.microsoft.com/windows/package-manager/winget/

Others (Cargo)
--------------

Cubic needs QEMU and its UEFI firmware on the host. Install them with your
package manager:

.. code-block::

    # Debian/Ubuntu
    sudo apt install qemu-system qemu-utils ovmf qemu-efi-aarch64
    # Fedora/RHEL
    sudo dnf install qemu-system-x86 qemu-img edk2-ovmf edk2-aarch64
    # Arch Linux
    sudo pacman -S qemu-full edk2-ovmf edk2-armvirt
    # openSUSE
    sudo zypper install qemu qemu-tools qemu-ovmf-x86_64 qemu-uefi-aarch64
    # macOS
    brew install qemu
    # Windows
    winget install SoftwareFreedomConservancy.QEMU

Install the `Rust toolchain`_ and then build Cubic:

.. code-block::

    rustup toolchain install stable
    cargo install cubic

.. _Rust toolchain: https://rustup.rs

Add the Cargo bin directory to your ``PATH`` on Linux:

.. code-block::

    echo 'export PATH="$PATH:$HOME/.cargo/bin"' >> ~/.profile
    source ~/.profile

On Linux, add your user to the ``kvm`` group for hardware acceleration. The
change becomes active after the next login:

.. code-block::

    sudo usermod -a -G kvm $USER

Verify the Install
------------------

.. code-block::

    cubic --help

.. _shell completions:

Shell Completions
-----------------

Cubic generates completion scripts for Bash, Zsh, Fish and PowerShell.
Regenerate the script after an update so it matches the installed commands and
options.

Bash
^^^^

.. code-block::

    mkdir -p ~/.local/share/cubic
    cubic completions bash > ~/.local/share/cubic/cubic.bash
    echo 'source ~/.local/share/cubic/cubic.bash' >> ~/.bashrc

Zsh
^^^

.. code-block::

    mkdir -p ~/.zfunc
    cubic completions zsh > ~/.zfunc/_cubic

Add ``fpath=(~/.zfunc $fpath)`` to ``~/.zshrc`` before its ``compinit`` call.

Fish
^^^^

.. code-block::

    mkdir -p ~/.config/fish/completions
    cubic completions fish > ~/.config/fish/completions/cubic.fish

PowerShell
^^^^^^^^^^

.. code-block::

    cubic completions powershell > $HOME\cubic.ps1

Add this line to the profile that ``$PROFILE`` names:

.. code-block::

    . $HOME\cubic.ps1

Start a new shell, type ``cubic`` and press Tab to check the setup.

Next Steps
----------

Continue with :ref:`create vm` and create your first virtual machine.
