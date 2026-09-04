# nostr-ln

The service side of **NNC** ([NIP-XX]) and **NWC** ([NIP-47]): grants,
limits, and the request pipeline. A node implements a handler; this crate
owns everything between a relay and that handler.

## Why it exists

NIP-XX's request pipeline existed **four times** in this project and no two
copies agreed. Five bugs are filed against one of them, including an
authorization bypass. Three codebases carry near-identical `usage_profile`
modules — one bug copied three times, not three bugs.

**A second implementation of a specification is a second interpretation of
it.**

## What it owns

| | |
|---|---|
| `grant` | kind `30198` — owner verification, `d` and `p` tags, idempotency, revocation, `OTHERS` |
| `profile` | `UsageProfile`: `methods`, `control`, `notifications`, `quota` |
| `limit` | `RateLimitRule` and its bucket — continuous refill, non-mutating checks |
| `subscription` | kind `30199` — what a controller wants, intersected with what its grant permits |

Mission 13.2 adds the handler traits, the dispatch and the seven-step
pipeline; 13.3 adds NIP-44 transport and the info events.

## The checks cannot be skipped

`VerifiedGrant` **cannot be constructed** without the owner set and the
node's own pubkey. A consumer cannot forget the check that
[dln-node#1] forgot — it was declared, written by a setter nobody called,
and never read — because the type it needs does not exist until the check
has run.

Absent configuration fails **closed**: an empty owner list refuses every
grant rather than accepting any.

## Three properties worth knowing

- **Deny by default, on all three maps.** Absent or empty grants nothing. A
  spending grant does not authorise an administrative call, and neither
  entitles a controller to be told anything.
- **Checking a limit never mutates.** Limits are evaluated at pipeline step
  4 and consumed at step 7, so a refused or failed request charges nothing.
- **A subscription authorizes nothing.** It says what a controller *wants*;
  its grant says what it *may have*; delivery is the intersection. Narrow
  the grant and delivery stops at once, without the subscription event
  changing — the node does not own that event and cannot delete it.

## Testing

Thirty-eight tests, no chain, no node and no relay:

```sh
cargo test
```

Several are a filed bug: [#1] owner verification, [#2] `rate` and not
`access_rate`, [#4] `since` advancing on withdrawal only, [#5] a rate that
can express one coin a week at all.

[NIP-XX]: https://github.com/DarkWebDivingClub/nips/blob/master/XX.md
[NIP-47]: https://github.com/nostr-protocol/nips/blob/master/47.md
[dln-node#1]: https://github.com/DarkWebDivingClub/dln-node/issues/1
[#1]: https://github.com/DarkWebDivingClub/dln-node/issues/1
[#2]: https://github.com/DarkWebDivingClub/dln-node/issues/2
[#4]: https://github.com/DarkWebDivingClub/dln-node/issues/4
[#5]: https://github.com/DarkWebDivingClub/dln-node/issues/5
