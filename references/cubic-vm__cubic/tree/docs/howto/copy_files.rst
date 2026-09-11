.. _copy files:

Copy Files
==========

``cubic scp`` copies files and directories between the host and a VM instance,
and between two VM instances. Both VM instances have to be running, because
``cubic scp`` does not start them for you.

Upload a File
-------------

.. code-block::

    $ cubic scp ./notes.txt demo:~/

Check that the file arrived:

.. code-block::

    $ cubic exec demo "cat ~/notes.txt"
    hello from the host

Download a File
---------------

.. code-block::

    $ cubic scp demo:~/result.txt ./result.txt
    $ cat ./result.txt
    made in the guest

Copy a Directory
----------------

.. code-block::

    $ cubic scp demo:~/Downloads .

Copy between Two VM Instances
-----------------------------

.. code-block::

    $ cubic scp demo:~/result.txt other:~/

A path is a guest path when it carries an instance name in front of the colon
and a host path when it does not, so the two arguments decide the direction of
the copy.

Related
-------

* :ref:`exec command` inside a VM instance
* :ref:`ssh connect`, which brings its own ``scp``
