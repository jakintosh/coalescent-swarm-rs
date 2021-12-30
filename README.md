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

## License

I am still researching the license I will use for this project. If you are wanting to use this software before I update with a real license reach out to me. My intention for this project is to promote and enable regenerative structures, both social and economic. Regardless of the specifics of the license I ultimately choose, I can guarantee that this software will not be licensed for use in extractive behaviours; this means it will not be licensed for use in for-profit applications, nor can it be forked or integrated into projects that will be used in for-profit applications.

I believe in the goals of "Free Software", but I also believe that from with an exploitation based system, we must be more aggressive and specific in how we use licensing to fight the intellectual property regime. This section serves as a statement of intention that this software is inherently political, and will not be licensed for use cases counter to that political goal.
