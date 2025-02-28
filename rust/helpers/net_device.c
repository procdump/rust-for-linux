// SPDX-License-Identifier: GPL-2.0

#include <linux/netdevice.h>
/* #include <net/net_trackers.h> */

struct net_device *rust_helper_get_net_device(struct net_device *dev)
{
	if (dev)
		dev_hold(dev);
	return dev;
}

void rust_helper_put_net_device(struct net_device *dev)
{
	dev_put(dev);
}

struct net_device *rust_helper_get_net_device_by_name(struct net *net, const char *name)
{
	return dev_get_by_name(net, name);
}
