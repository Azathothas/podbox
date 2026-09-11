.. _exec command:

Run a Command
=============

``cubic exec`` runs one command inside a VM instance and prints its output on
the host. A stopped VM instance is started first, so a single command is enough
to get an answer out of a guest.

Run a Command
-------------

Put ``--`` between the instance and the command:

.. code-block::

    $ cubic exec demo -- uname -sr
    Linux 7.0.0-30-generic

Everything after the instance goes to the guest, so a word that starts with a
hyphen arrives as it is:

.. code-block::

    $ cubic exec demo -- echo -ne hello
    hello

Three Ways to Write It
----------------------

Cubic joins the words of the command with a space and hands the result to the
shell of the guest, so these three lines send the same string and all three
work:

.. code-block::

    $ cubic exec demo -- echo -ne hello
    $ cubic exec demo echo -ne hello
    $ cubic exec demo "echo -ne hello"

The ``--`` form says most clearly where the command starts, so the rest of this
page uses it. Leaving ``--`` out reads well for a short command. The quoted form
is the one older scripts use and it keeps working.

Options of Cubic itself have to come before the command, because everything
after the instance belongs to the guest:

.. code-block::

    $ cubic exec demo --env GITHUB_TOKEN -- pip install my-lib

Run Several Commands
--------------------

The command reaches the guest through its shell. Quote the whole command to
keep the operators away from the shell of the host:

.. code-block::

    $ cubic exec demo "sudo apt update && sudo apt install -y vim"

Quote it as well when a word holds a space, or when the host would expand
something you want the guest to see, because the words arrive as one string:

.. code-block::

    $ cubic exec demo "grep 'hello world' /etc/motd"
    $ cubic exec demo "echo \$HOME"

Check the Result
----------------

``cubic exec`` ends with the exit code of the command in the guest, so a script
on the host can act on it:

.. code-block::

    $ cubic exec demo -- test -f /etc/motd
    $ echo $?
    0
    $ cubic exec demo -- test -f /etc/cubic-tutorial
    $ echo $?
    1

The code 255 means the session ended without a result from the guest, for
example when the connection broke or when you left the session with Enter,
``~``, ``.``.

``cubic ssh`` and ``cubic run`` work the same way and end with the exit code of
the login shell in the guest.

A command that reads from a pipe or a file works as well, because Cubic asks for
a terminal in the guest only when your own terminal is attached:

.. code-block::

    $ echo hello | cubic exec demo -- cat
    hello

Run a Command at Creation
-------------------------

``cubic exec`` runs whenever you like and needs a guest that already exists.
``cubic create --exec`` is the other one, a command cloud-init runs as root once
on the first boot, which is where package installs for a fresh VM instance
belong:

.. code-block::

    $ cubic create builder --image ubuntu --exec "apt install -y build-essential"

:ref:`template file` sets the same thing with the ``run`` list.

Related
-------

* :ref:`forward env vars` of the host into a VM instance
* :ref:`copy files` in and out of a VM instance
* :ref:`template file` to run commands on the first boot of every VM instance
