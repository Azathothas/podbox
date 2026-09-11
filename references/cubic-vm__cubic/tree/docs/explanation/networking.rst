.. _networking:

Networking
==========

A VM instance gets its network from QEMU user mode networking. QEMU implements a
small TCP/IP stack inside its own process and speaks to the outside through
ordinary sockets of your user account. There is no bridge, no tap device and no
change to the network configuration of the host.

A bridge would need elevated privileges and would reconfigure the host network.
User mode networking needs neither, works the same on Linux, macOS and Windows,
and cannot affect anything outside the QEMU process.

What the Guest Sees
-------------------

QEMU hands the guest a private network by DHCP. The addresses are the QEMU
defaults and every VM instance sees the same ones:

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Address
     - Role
   * - ``10.0.2.15``
     - The guest itself.
   * - ``10.0.2.2``
     - Gateway. Traffic to it leaves through the host.
   * - ``10.0.2.3``
     - DNS server, which QEMU answers from the resolver of the host.

These addresses are internal to one VM instance. Two VM instances both see
``10.0.2.15`` for themselves and neither can reach the other by address.

Outbound Traffic Works, Inbound Does Not
----------------------------------------

A guest can open connections to the outside on its own, which is what package
installs and downloads need. Nothing has to be configured for that.

The other direction does not work by itself. The guest has no address on the
network of the host, so nothing on the host and nothing on your LAN can reach a
port inside the guest. To reach a service in a guest you ask QEMU to forward a
host port to a guest port, which is what ``--port`` does. :ref:`port forward`
shows how.

Cubic always forwards one port of its own, from a free host port to port 22 of
the guest, because SSH is how every other command reaches the guest.

A forward binds to ``127.0.0.1`` unless you name another address. That keeps the
service on loopback, where only your own computer can reach it. Binding to
``0.0.0.0`` publishes the service to your whole network, so do it only on
purpose. :ref:`security` explains what that gives up.

Why Ping Often Fails in a Guest
-------------------------------

User mode networking carries TCP and UDP. ICMP is a different matter, because
sending an ICMP echo normally needs a raw socket, and a raw socket needs
privileges QEMU deliberately does not have.

QEMU works around this where the operating system allows an unprivileged ping
socket, and on Linux that depends on the ``net.ipv4.ping_group_range`` setting
of the host. So ``ping 8.8.8.8`` inside a guest may fail while ``curl`` to the
same host works. A failing ping is not proof that the guest is offline, so test
with a TCP connection instead:

.. code-block::

    $ curl -sS https://example.com > /dev/null && echo online

Isolation
---------

``--isolate`` turns on the ``restrict`` mode of QEMU user mode networking. The
guest can then reach neither the internet, nor your LAN, nor the host itself.
Packets are dropped inside the QEMU process, so the isolation does not depend on
a firewall of the host being configured correctly.

Explicit port forwards keep working, and that includes the SSH port Cubic sets
up. An isolated VM instance is therefore still reachable with ``cubic ssh``,
``cubic exec`` and ``cubic scp``, which is what makes isolation usable rather
than a dead end. Use it for untrusted code or for a build that must not reach
the network.

Isolation can be switched at any time with ``cubic modify --isolate`` and
``cubic modify --no-isolate``. The change applies when the VM instance restarts.

Guests Do Not See Each Other
----------------------------

Every VM instance has its own QEMU process with its own network stack, so guests
are separated from each other by construction. Two VM instances that need to
talk have to go through the host. Forward a port of the first one and let the
second one connect to the gateway address ``10.0.2.2``, which reaches the host.

Related
-------

* :ref:`port forward` to reach a service inside a guest
* :ref:`first isolation` for a VM instance without network access
* :ref:`security` for what loopback binding does and does not protect
* :ref:`the machine` for the network device itself
