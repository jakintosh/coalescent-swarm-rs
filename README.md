# Coalescent Swarm

## About

P2P Client-Swarm infrastructure for the Coalescent Computer.

Client-Swarm means a swarm of semi-dedicated "servers" (a mac mini, a raspberry pi, a gaming tower) do the shared heavy lifting of networking and data storage in a p2p mesh network, while individual clients (a smartphone or tablet, or a user facing app on the server itself) communicate with known authorized servers. The intention is for the end result to be a relatively unopinionated and highly configurable p2p networking tool for the [Coalescent Computer](https://github.com/jakintosh/coalescent-computer-protocol): think Plex Media Server but data/app agnostic, and also the host computer is part of a p2p swarm.

The goals:
- allow users to plug in public-key identities
- connect to (and act as) bootstrap server for IP discovery/hole-punching
- allow authorized client devices to route through
- kademlia DHT to share peer addresses and metadata
- probably encryption between peers, signed data, maybe RPCs... we'll see