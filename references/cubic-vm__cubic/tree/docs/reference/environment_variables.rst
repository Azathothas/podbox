.. _env vars:

Environment Variables
=====================

Cubic reads the following environment variables from the host. None of them is
required. Each one either overrides a default or tells Cubic something about the
host it runs on.

To pass a variable of the host into a VM instance instead, see
:ref:`forward env vars`.

QEMU and Firmware
-----------------

These variables override the automatic detection described in
:ref:`qemu detection`.

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Variable
     - Description
   * - ``CUBIC_QEMU_DIR``
     - Directory of the QEMU install. Placed at the front of the search list for
       the QEMU binaries, and used to locate the ``share/qemu/firmware``
       descriptors.
   * - ``CUBIC_QEMU_FW_AMD64``
     - Path to a specific amd64 UEFI firmware (CODE) file. Overrides the
       descriptor based firmware selection for amd64 VM instances.
   * - ``CUBIC_QEMU_FW_ARM64``
     - Path to a specific arm64 UEFI firmware (CODE) file. Overrides the
       descriptor based firmware selection for arm64 VM instances.
   * - ``PATH``
     - Searched for ``qemu-system-x86_64``, ``qemu-system-aarch64`` and
       ``qemu-img`` after ``CUBIC_QEMU_DIR`` and before the built in default
       directories.
   * - ``XDG_CONFIG_HOME``
     - Adds ``$XDG_CONFIG_HOME/qemu/firmware`` to the firmware descriptor
       search, falling back to ``$HOME/.config/qemu/firmware``. This variable
       points at the QEMU configuration, not at any Cubic configuration.

File Locations
--------------

These variables decide where Cubic keeps VM instances and downloaded images.
:ref:`file locations` shows the resulting paths.

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Variable
     - Description
   * - ``HOME``
     - Base directory on macOS. Fallback for the data and cache directory on
       Linux. Also the ``.ssh`` directory Cubic searches for your SSH public
       keys.
   * - ``XDG_DATA_HOME``
     - Directory of the VM instances on Linux. Falls back to
       ``$HOME/.local/share``.
   * - ``XDG_CACHE_HOME``
     - Directory of the image cache on Linux. Falls back to ``$HOME/.cache``.
   * - ``LOCALAPPDATA``
     - Directory of the VM instances on Windows. Required on Windows.
   * - ``TEMP``
     - Directory of the image cache on Windows. Required on Windows.
   * - ``SNAP_USER_COMMON``
     - Directory of the VM instances in the snap package. Takes precedence over
       ``XDG_DATA_HOME``. The snap sets it for you.
   * - ``SNAP_REAL_HOME``
     - Real home directory of the user in the snap package. Its ``.ssh``
       directory is searched for your SSH public keys. The snap sets it for you.
   * - ``SNAP``
     - Marks a run inside the snap package. Cubic then suggests
       ``sudo snap connect cubic:kvm`` when hardware acceleration is missing.
       The snap sets it for you.

User and Terminal
-----------------

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Variable
     - Description
   * - ``USER``, ``LOGNAME``, ``USERNAME``
     - Default user name of a new VM instance. The first variable that is set
       wins. A value of ``root`` or an unusable name falls back to ``cubic``.
       Use ``cubic create --user`` to set the name per VM instance.
   * - ``TERM``
     - Terminal type requested for the SSH session of ``cubic ssh`` and
       ``cubic exec``. Falls back to ``xterm``.
   * - ``NO_COLOR``
     - Turns off colored output when it is set to any value, including an empty
       one. See `no-color.org <https://no-color.org>`_.

Network Proxy
-------------

Cubic downloads images over HTTPS and honors the usual proxy variables. Each
name also works in lower case, such as ``https_proxy``. The upper case name wins
when both are set.

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Variable
     - Description
   * - ``HTTPS_PROXY``
     - Proxy URL for HTTPS requests, such as ``http://proxy.example.com:3128``.
       This is the one image downloads use.
   * - ``HTTP_PROXY``
     - Proxy URL for plain HTTP requests.
   * - ``ALL_PROXY``
     - Proxy URL for both. A matching ``HTTP_PROXY`` or ``HTTPS_PROXY`` wins.
   * - ``NO_PROXY``
     - Comma separated list of hosts that bypass the proxy, such as
       ``localhost,10.0.0.0/8``. A single ``*`` bypasses the proxy for every
       host.

``SSL_CERT_FILE`` and ``SSL_CERT_DIR`` have no effect, because Cubic ships its
own root certificates. :ref:`use a proxy` covers the platform differences and
what to do when a proxy terminates TLS itself.

Variables Cubic Sets
--------------------

Cubic sets these variables for the QEMU process it starts. Setting them on the
host has no effect.

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - Variable
     - Description
   * - ``PATH``
     - Search list Cubic built for the QEMU binaries.
   * - ``QEMU_MODULE_DIR``
     - Module directory of the QEMU install Cubic found.

Related
-------

* :ref:`qemu detection` and how the overrides fit in
* :ref:`file locations` of VM instances and images
* :ref:`forward env vars` of the host into a VM instance
* :ref:`use a proxy` for image downloads
