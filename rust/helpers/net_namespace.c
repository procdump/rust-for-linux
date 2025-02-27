// SPDX-License-Identifier: GPL-2.0

#include <linux/nsproxy.h>
#include <net/net_namespace.h>

struct net *rust_helper_get_net_ns(struct net *ns)
{
	return get_net(ns);
}

void rust_helper_put_net_ns(struct net *ns)
{
	put_net(ns);
}

/* Get a reference to task's net namespace bumping the refcount. */
struct net *rust_helper_task_get_net_ns(struct task_struct *tsk)
{
	/*
	pid_t pid = tsk->pid;
	struct net *net = get_net_ns_by_pid(pid);
	*/
	struct net *net = tsk->nsproxy->net_ns;
	if (net)
		get_net(net);
	return net;
}

/* Get a reference to task's net namespace w/o bumping the refcount. */
struct net *rust_helper_task_net_ns(struct task_struct *tsk)
{
	struct net *net = tsk->nsproxy->net_ns;
	return net;
}
