# Current Milestone

## Active Milestone

**Milestone 1: Native BGZF Core**

See:

* [milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)

## Why This Is Current

Native BGZF ownership is the physical substrate for BAM. It enables:

* EOF checks without external parser dependence
* controlled block reading
* future virtual-offset handling
* the reader and writer foundation used by later BAM milestones

## What “Done” Means

For contributors, Milestone 1 is done only when:

* BGZF block reading is Bamana-native and exercised by tests
* BGZF EOF behavior is Bamana-native and tested
* BGZF writing is sufficient for BAM-compatible output foundations
* benchmark hooks for read, write, and EOF latency are defined and runnable
* no production BGZF hot path depends on `noodles`

## What Should Not Happen

Do not skip ahead to command-level rewrites that assume a mature native scanner
or header codec before the BGZF substrate is clearly owned.
