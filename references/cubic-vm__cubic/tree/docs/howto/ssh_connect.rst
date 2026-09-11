.. _ssh connect:

Use the Host SSH Client
=======================

``cubic ssh`` is the short way into a guest. The SSH client of your host works
just as well, which is what you want for an editor, an agent or a tool that
speaks SSH on its own.

Use the Key Cubic Made
----------------------

Every VM instance already has its own key and a forwarded SSH port.
``cubic show --all`` prints the command that uses both:

.. code-block::

    $ cubic show --all demo
    [...]
    SSH Port:     40881
    [...]
    SSH Key:      ~/.local/share/cubic/machines/demo/ssh_client_key
    SSH:          ssh -i ~/.local/share/cubic/machines/demo/ssh_client_key -p 40881 alice@localhost

Copy that line and you are in:

.. code-block::

    $ ssh -i ~/.local/share/cubic/machines/demo/ssh_client_key -p 40881 alice@localhost
    alice@demo:~$

The VM instance has to be running, because the host client does not start it.
Use ``cubic start demo`` first.

Use Your Own Key
----------------

Add your own public key to the guest when you would rather use the key of your
agent, or when a tool cannot be told which key file to take:

.. code-block::

    $ cubic ssh demo
    alice@demo:~$ mkdir -p ~/.ssh && echo '<your-public-key>' >> ~/.ssh/authorized_keys
    alice@demo:~$ exit

Your public key is usually ``~/.ssh/id_ed25519.pub`` on the host. Both keys work
side by side afterwards.

Give the VM Instance a Fixed Port
---------------------------------

Cubic picks a free SSH port for each VM instance, and it can pick a new one when
the old port is taken at the next start. Add a forward of your own for an entry
in ``~/.ssh/config`` that should keep working:

.. code-block::

    $ cubic modify demo --port 2222:22

A running VM instance opens the host port right away, so no restart is needed.
The entry then stays valid:

.. code-block::

    Host demo
        HostName localhost
        Port 2222
        User alice
        IdentityFile ~/.local/share/cubic/machines/demo/ssh_client_key

With that in place ``ssh demo``, ``scp`` and ``rsync`` all reach the guest.

Related
-------

* :ref:`copy files` with ``cubic scp`` instead
* :ref:`port forward` to reach other services in the guest
* :ref:`file locations` of the SSH key of a VM instance
* :ref:`console login` reaches a guest whose SSH server stopped working
