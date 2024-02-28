sudo brctl addbr br10
sudo brctl addbr br20
sudo brctl addif br10 enx48225411bf0f
sudo brctl addif br20 enx00e04c534458

# vng -b LLVM=-15 -v
# start the VM
# /usr/bin/qemu-system-x86_64 -name virtme-ng -m 1G -fsdev local,id=virtfs5,path=/,security_model=none,readonly=on,multidevs=remap -device virtio-9p-pci,fsdev=virtfs5,mount_tag=/dev/root -machine accel=kvm:tcg -device i6300esb,id=watchdog0 -cpu host -parallel none -net none -echr 1 -serial none -chardev stdio,id=console,signal=off,mux=on -serial chardev:console -mon chardev=console -vga none -display none -smp 8 -device virtio-net-pci,netdev=n1 -netdev bridge,id=n1,br=br10 -device virtio-net-pci,netdev=n2 -netdev bridge,id=n2,br=br20 -kernel ./arch/x86/boot/bzImage -append 'virtme_hostname=virtme-ng virtme_link_mods=/home/boris/projects/virtme-ng/linux/.virtme_mods/lib/modules/0.0.0 virtme_rw_overlay0=/etc virtme_rw_overlay1=/lib virtme_rw_overlay2=/home virtme_rw_overlay3=/opt virtme_rw_overlay4=/srv virtme_rw_overlay5=/usr virtme_rw_overlay6=/var console=ttyS0 psmouse.proto=exps "virtme_stty_con=rows 49 cols 254 iutf8" TERM=xterm-256color virtme.dhcp net.ifnames=0 biosdevname=0 virtme_chdir=home/boris/projects/virtme-ng/linux virtme_user=boris rootfstype=9p rootflags=version=9p2000.L,trans=virtio,access=any raid=noautodetect ro init=/home/boris/.local/lib/python3.10/site-packages/virtme/guest/bin/virtme-ng-init'

