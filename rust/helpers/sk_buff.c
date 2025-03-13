// SPDX-License-Identifier: GPL-2.0

#include <linux/skbuff.h>

void rust_helper_kfree_skb(struct sk_buff *skb)
{
	kfree_skb(skb);
}

struct sk_buff *rust_helper_skb_get(struct sk_buff *skb)
{
	return skb_get(skb);
}

unsigned int rust_helper_skb_headlen(const struct sk_buff *skb)
{
	return skb_headlen(skb);
}