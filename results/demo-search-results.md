# Demo: Search Tool

Interactive search tool for querying the shipment database.

## Usage

```
Usage: demo_search <command> [args]

Commands:
  carrier <id>     - Search shipments by carrier (partial match)
  lane <pattern>   - Search lanes (e.g., 'DFW' or 'DFW->AUS')
  origin <zip3>    - Search shipments from origin DC
  dest <zip3>      - Search shipments to delivery region
  late             - Show recent late shipments
  early            - Show recent early shipments
  long             - Show longest transit times
  stats            - Quick stats summary

Examples:
  ./target/release/demo_search carrier xpo
  ./target/release/demo_search origin 750
  ./target/release/demo_search late
```

## Example: Recent Late Shipments

```
Recent LATE shipments...

Load ID         Mode         Carrier              Route          Goal Actual   Status
---------------------------------------------------------------------------------------
654e42f585fe    LTL          XPO Logistics        CLE→030           2     69 Late (+67)
d9af42d3768f    LTL          XPO Logistics        DFW→874           2     68 Late (+66)
a4fdc046e8e1    Truckload    R+L Carriers         MKE→DFW           2     64 Late (+62)
000783b2f394    LTL          XPO Logistics        DFW→MIA           3     61 Late (+58)
7734dfb6634e    LTL          XPO Logistics        DFW→591           3     56 Late (+53)
a4458c22754a    LTL          XPO Logistics        DFW→585           3     53 Late (+50)
c6cc20a8698c    LTL          XPO Logistics        DFW→591           3     48 Late (+45)
28f894ef2054    LTL          XPO Logistics        DFW→ASH           2     47 Late (+45)
abddf0336d8e    LTL          XPO Logistics        DFW→RDU           3     45 Late (+42)
641464bc2099    LTL          XPO Logistics        DFW→827           3     42 Late (+39)
de84353bda88    LTL          XPO Logistics        DFW→DEN           2     41 Late (+39)
fd75ad2227cd    LTL          XPO Logistics        DFW→577           5     39 Late (+34)
bcb1d678b612    TLFlatbed    Dayton Freight       CLE→049           2     36 Late (+34)
554ed2ad1638    LTL          XPO Logistics        DFW→TUS           2     34 Late (+32)
3dadf69fbcf9    LTL          XPO Logistics        DFW→LAS           7     33 Late (+26)
5ecefa58f043    LTL          XPO Logistics        DFW→HBG           3     33 Late (+30)
662b0e73ebde    LTL          XPO Logistics        DFW→EVV           3     30 Late (+27)
c1506fe705f7    LTL          XPO Logistics        DFW→CDR           2     30 Late (+28)
da3f86b75c9d    LTL          XPO Logistics        DFW→ELP           2     29 Late (+27)
0cc35e791e86    LTL          XPO Logistics        CLE→TRT           2     27 Late (+25)
01d73a5449fa    LTL          XPO Logistics        DFW→MUS           1     27 Late (+26)
0fefa2ca9097    LTL          Southeastern Freight STN→DFW           2     26 Late (+24)
b223347b05c0    LTL          XPO Logistics        DFW→591           3     26 Late (+23)
e866d1049db3    LTL          XPO Logistics        DFW→ATL           2     26 Late (+24)
540bb843b2b6    LTL          XPO Logistics        DFW→REN           6     26 Late (+20)
```

## Example: Quick Stats

```
=== QUICK STATS ===

Total Shipments:         72965
On-Time:                 46609 (63.9%)
Late:                    13998 (19.2%)
Early:                   12358 (16.9%)
Avg Transit Days:          2.9
```
