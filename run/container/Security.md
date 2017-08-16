# Installation security notes


The container binary needs capabilities for:

  - Configuring namespaces.

  - Configuring cgroup via systemd.  Effective uid is temporarily set to root.

  - Setting supplementary groups in user namespace.


Things controlled by the user who can execute the container binary:

  - Choose any two pairs of the parent namespace's user and group ids to be
    mapped to the container's user namespace.  The credentials are used to (1)
    set up the mount namespace, and (2) for the executor process and its
    children.

  - Choose one of the parent namespace's group ids to be mapped to the
    container's user namespace.  It is used as a supplementary group of the
    contained processes, which need it for opening files in /proc/self/fd/.

  - Choose any cgroup as the parent for the container's cgroup.

  - Supply the file descriptor used for interacting with the executor process
    inside the container.  It can be used to spawn and kill processes inside
    the container, and execute arbitrary code in the processes.


Environmental factors:

  - The container binary needs CAP_DAC_OVERRIDE, CAP_SETGID, and CAP_SETUID
    capabilities.  It should be executable only by a single, trusted user.

  - The binaries executed inside the container are determined by the location
    of the container binary itself: it looks for the "executor" and "loader"
    files in the same directory where it is located.  The write permissions of
    the directory and the binaries should be limited.  (Note that executor and
    loader don't need capabilities, and they need to have more relaxed read and
    execution permissions.)

  - Cgroup configuration needs to be via systemd.  By default a container
    instance gets its own cgroup under system.slice, but that's it.
