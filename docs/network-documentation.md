# Network Documentation

## VLANs

### Management (1)
Trunked to Red Trunk Line and AP on Blue Switch

more tbd

### Red 1 Driverstation (10)
Trunked to AP, Keystone, and Red Trunk Line on Blue Switch

Trunked in from Input Trunk Line and untagged on Red 1 DS port

Keystone interface: eth0.10

Network: 10.xx.yy.0/24


### Red 2 Driverstation (20)
Trunked to AP, Keystone, and Red Trunk Line on Blue Switch

Trunked in from Input Trunk Line and untagged on Red 2 DS port

Keystone interface: eth0.20

Network: 10.xx.yy.0/24


### Red 3 Driverstation (30)
Trunked to AP, Keystone, and Red Trunk Line on Blue Switch

Trunked in from Input Trunk Line and untagged on Red 3 DS port

Keystone interface: eth0.30

Network: 10.xx.yy.0/24


### Blue 1 Driverstation (40)
Trunked to AP and Keystone and untagged on Blue 1 DS on Blue Switch

Keystone interface: eth0.40

Network: 10.xx.yy.0/24


### Blue 2 Driverstation (50)
Trunked to AP and Keystone and untagged on Blue 1 DS on Blue Switch

Keystone interface: eth0.50

Network: 10.xx.yy.0/24


### Blue 3 Driverstation (60)
Trunked to AP and Keystone and untagged on Blue 1 DS on Blue Switch

Keystone interface: eth0.60

Network: 10.xx.yy.0/24


### FMS (100)
Trunked to AP, Keystone, and Red Trunk Line and untagged to Display Device and Admin Device on Blue Switch

Keystone interface: eth0.100

Network: 10.0.100.0/24


## DHCP

### Robot VLANs (10-60)
(dev note: may not be needed, ap may do this for us)
- IP Range: 10.xx.yy.100 - 10.xx.yy.150
- Subnet: /24
- Lease: 24h
- DNS: Keystone
- Gateway: 10.xx.yy.4 (dev note: might also be keystone, not sure yet)

### LAN (100)
- IP Range: 10.0.100.200 - 10.0.100.253
- Subnet: /24
- Lease: 24h
- DNS: Keystone (10.0.100.5)
- Gateway: Keystone (10.0.100.5)


## DNS

dnsmasq

Rules on Keystone determine whether answers locally or forwards to 8.8.8.8 / 1.1.1.1


## Firewall and Routing

### Input chain (Traffic to Keystone)

Drop everything

Allowed:
- From internal VLANS:
    - DHCP (67/68)
    - DNS (53)
    - ICMP
- From LAN:
    - SSH (22)
    - HTTP/HTTPS (80/443)
- From WAN:
    - DHCP replies (67/68)


### Forward chain (Traffic through Keystone)

Drop everything

Allowed:
- LAN -> WAN
    - Full internet access
    - NAT kicks in
- Robot VLANs -> WAN
    - Dev mode (dev note: second stage, implement later)
        - Full internet access
        - NAT kicks in
    - Normal mode (dev note: default, first step)
        - No traffic
- Robot VLANs -> LAN
    - UDP 1145
    - UDP 1160
    - TCP 175
    - Anything going to 10.*.*.4


### Output chain (Traffic from Keystone)

Everything allowed


## NAT
postrouting tool

`oifname $WAN_IFACE masquerade` 

All traffic leaving comes from Keystone IP but get's NAT'd so it can come back


## Routing
All ipv4 enabled ip forwarding