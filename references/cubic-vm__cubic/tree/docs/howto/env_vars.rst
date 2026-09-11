.. _forward env vars:

Forward Environment Variables
=============================

``--env`` passes an environment variable of the host into a VM instance, so a
build or a script inside the guest can read it without the value ever being
written to disk in the guest.

This page is about variables you pass into a guest. Cubic also reads variables
of the host for its own settings, such as ``CUBIC_QEMU_DIR`` and the proxy of
the image download. :ref:`env vars` lists all of them.

Forward a Variable of the Host
------------------------------

Pass the variable by name and Cubic reads its value from your host
environment:

.. code-block::

    $ export GITHUB_TOKEN=ghp_yourTokenHere
    $ cubic ssh builder --env GITHUB_TOKEN

Confirm inside the guest that the value arrived:

.. code-block::

    $ echo $GITHUB_TOKEN
    ghp_yourTokenHere

The variable exists for the length of the session only.

Pass an Explicit Value
----------------------

Use ``KEY=VALUE`` when the value is not set on the host:

.. code-block::

    $ cubic ssh builder --env BUILD_TARGET=release

Both forms can be repeated and combined:

.. code-block::

    $ cubic ssh builder --env GITHUB_TOKEN --env BUILD_TARGET=release

Use It for a Build
------------------

A forwarded token lets a build fetch private dependencies without storing the
token in the guest:

.. code-block::

    $ git config --global url."https://${GITHUB_TOKEN}@github.com/".insteadOf "https://github.com/"
    $ pip install git+https://github.com/your-org/private-lib.git

Run a One-Off Command
---------------------

``cubic exec`` takes the same flag and needs no interactive shell:

.. code-block::

    $ cubic exec builder --env GITHUB_TOKEN -- pip install git+https://github.com/your-org/private-lib.git

Related
-------

* :ref:`exec command` in a VM instance
* :ref:`env vars` that Cubic itself reads
* :ref:`use a proxy` for image downloads
