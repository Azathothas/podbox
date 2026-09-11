.. _recover disk:

Recover Data from a Disk Image
==============================

This guide gets files out of a VM instance that no longer boots or no longer
answers ``cubic ssh``. The disk of a VM instance is a plain ``qcow2`` image, so
``qemu-img`` and ``qemu-nbd`` read it straight from the host.

Stop the VM Instance
--------------------

Mount the image only while the VM instance is stopped, so the host and the
guest never write to it at the same time:

.. code-block::

    $ cubic stop demo --wait

Locate the Disk Image
---------------------

The disk of a VM instance is ``machine.img`` in its instance directory, on
Linux typically ``~/.local/share/cubic/machines/demo/machine.img``.
:ref:`file locations` lists the directory of every platform.

Copy Files with ``virt-cat`` on Linux
-------------------------------------

``libguestfs-tools`` reads the image without mounting it, which is the quickest
route when the package is available:

.. code-block::

    $ virt-filesystems -a ~/.local/share/cubic/machines/demo/machine.img --all --long -h
    $ virt-cat -a ~/.local/share/cubic/machines/demo/machine.img /etc/hostname
    $ virt-copy-out -a ~/.local/share/cubic/machines/demo/machine.img /home/alice/work .

``virt-copy-out`` writes the listed guest paths into the current host
directory. All three tools open the image read only, so the guest filesystem
stays as it is.

Mount the Image with ``qemu-nbd`` on Linux
------------------------------------------

The Network Block Device kernel module exposes the disk as a block device
instead. ``qemu-nbd`` comes with the ``qemu-utils`` package of your
distribution, which the Snap package does not ship:

.. code-block::

    $ sudo modprobe nbd max_part=8
    $ sudo qemu-nbd --read-only --connect=/dev/nbd0 \
        ~/.local/share/cubic/machines/demo/machine.img
    $ lsblk /dev/nbd0

Mount the guest root partition (typically ``/dev/nbd0p1``) somewhere on the
host and copy files out:

.. code-block::

    $ sudo mkdir -p /mnt/cubic
    $ sudo mount -o ro /dev/nbd0p1 /mnt/cubic
    $ cp -a /mnt/cubic/home/alice/work ~/recovered/

Keep ``-o ro`` on the mount, so the guest filesystem stays untouched.

Detach the Image
----------------

When the recovery is done, unmount and disconnect the device:

.. code-block::

    $ sudo umount /mnt/cubic
    $ sudo qemu-nbd --disconnect /dev/nbd0

Read the Disk on macOS and Windows
----------------------------------

``virt-cat`` and ``qemu-nbd`` are Linux tools. On macOS and Windows convert the
disk to a raw image with ``qemu-img``, which comes with QEMU everywhere, and
open the result with the disk tools of your system:

.. code-block::

    $ qemu-img convert -O raw \
        ~/Library/cubic/machines/demo/machine.img machine.raw

A Linux VM instance can do the work as well. Copy the image into one with
``cubic scp`` and use the tools above inside that guest.

Related
-------

* :ref:`file locations` names every file of a VM instance
* :ref:`console login` reaches a guest that still boots
* :ref:`qemu not found` helps when no VM instance starts at all
