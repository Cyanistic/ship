# What exact command and attachment contract completes the first slice?

ID: 04
Parent: [Plan the first client/server terminal slice](../map.md)
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: unassigned
Blocked by: 02, 03

## Question

Given the accepted supplied-session capture direction and default whole-vector diff strategy, what minimal command/response/SSE contract and process lifecycle make one terminal usable across separate client/server processes?

Settle terminal creation/identity, key/text/paste payloads, absolute view dimensions, snapshot/subscription coordination, instance/base/new revisions, reconnect and distinct close/disconnect/server-stop outcomes. Confirm HTTP/PTY integration choices and explicit startup/cleanup behavior. Settle coherent publication despite separately locked size/buffer/cursor reads; the stable-output probe did not establish atomic capture. Specify first-slice manual completion evidence without expanding into automation catalog, remote features, auth, configurable keybinding/reload systems, plugin infrastructure, speculative diff optimization or permanent tests. Minimal fixed operational/exit bindings are in scope; their exact spelling is still open. Use grilling and domain-modeling after claiming; Herdr source references in the planning notes are evidence, not a mandatory API. This ticket produces decisions and a handoff, not implementation.
