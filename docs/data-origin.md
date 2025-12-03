# Data origin

The data strongly suggests DC → Final Delivery Region, not terminal-to-terminal:

 Evidence

  | Metric                               | Origins       | Destinations              |
  |--------------------------------------|---------------|---------------------------|
  | Unique ZIP3s                         | 477           | 793                       |
  | Top 3 concentration                  | 61% of volume | 18% of volume             |
  | Low-volume locations (<10 shipments) | Very few      | 301 (38% of destinations) |

## Pattern Analysis

  Origins are concentrated (Distribution Centers):
  - 750xx (Dallas): 19,825 shipments (27%)
  - 172xx (Harrisburg): 12,571 (17%)
  - 441xx (Cleveland): 12,333 (17%)

  Destinations are dispersed (Final Delivery Regions):
  - Dallas DC (750xx) ships to: Tucson, Las Vegas, Miami, Atlanta, SLC, etc.
  - 301 destination ZIP3s receive fewer than 10 shipments
  - Long tail distribution typical of e-commerce/retail delivery

## Conclusion

  This is outbound fulfillment data:
  - Origin: Warehouse/DC locations (few, high-volume hubs)
  - Destination: Customer delivery regions (many, dispersed ZIP3s)

  It's called "last-mile" data because it tracks the final leg from DC to customer region, though true last-mile would be ZIP5/address level. The ZIP3 precision likely aggregates to
  protect customer PII while still enabling regional analysis.

    Not terminal-to-terminal because:
  1. Destinations are too dispersed (793 vs typical ~50-100 LTL terminals)
  2. Long tail of low-volume destinations (real customer regions)
  3. Pattern matches outbound distribution, not linehaul network

## What's In The Data

  | Field          | Contains               | Example      |
  |----------------|------------------------|--------------|
  | origin_zip_3d  | ZIP3 region only       | 441xx        |
  | dest_zip_3d    | ZIP3 region only       | 172xx        |
  | carrier_pseudo | Anonymized carrier ID  | 19936bf01cc6 |
  | load_id_pseudo | Anonymized shipment ID | 786caec8eb1b |
  | lane_id        | Anonymized lane ID     | 109c918ef6db |

## What's Missing

  - ❌ Facility name (e.g., "Dallas DC #3")
  - ❌ Facility ID / code
  - ❌ Facility type (DC, terminal, cross-dock)
  - ❌ Company/shipper identity
  - ❌ Consignee name/type (retail store, residential, business)

## What We Can Infer (but not prove)

  Based on volume patterns, the top origins are likely DCs:

  | ZIP3  | Location       | Volume | Likely Role   |
  |-------|----------------|--------|---------------|
  | 750xx | Dallas, TX     | 19,825 | Major DC      |
  | 172xx | Harrisburg, PA | 12,571 | East Coast DC |
  | 441xx | Cleveland, OH  | 12,333 | Midwest DC    |

  But we can't distinguish:
  - Multiple DCs in the same ZIP3
  - Whether 750xx is 1 mega-DC or 3 smaller facilities
  - Shipper identity (could be one company or many)

  The data was clearly anonymized/pseudonymized for privacy, stripping facility identities while preserving geographic and operational patterns.

## Facility Types

  | Term                     | What It Is                                                 | Typical Use                           |
  |--------------------------|------------------------------------------------------------|---------------------------------------|
  | DC (Distribution Center) | Large warehouse for storing & shipping to customers/stores | Retail/e-commerce fulfillment         |
  | Fulfillment Center (FC)  | DC optimized for e-commerce direct-to-consumer             | Amazon, online retailers              |
  | Warehouse                | General storage facility                                   | Broader term, any storage             |
  | Terminal                 | Carrier facility for consolidation/deconsolidation         | LTL carriers (break-bulk)             |
  | Cross-dock               | Transfer point, no long-term storage                       | Quick transfer between trucks         |
  | Hub                      | Central node in hub-and-spoke network                      | Airlines, parcel carriers (UPS/FedEx) |
  | Break-bulk               | Where LTL shipments are sorted/transferred                 | Same as terminal for LTL              |
  | Service Center           | Terminal (carrier-preferred term)                          | XPO, Estes, Old Dominion              |
  | Relay Point              | Driver swap location for long-haul                         | Truckload operations                  |
  | Pool Point               | Regional consolidation for final delivery                  | LTL zone-skip optimization            |

## In Your Data Context

  Given the pattern we see (concentrated origins → dispersed destinations):

  - Origins → Best called "DCs" or "Fulfillment Centers"
  - Destinations → Best called "Delivery Regions" or "Consignee Locations"

  If this were terminal-to-terminal LTL data, you'd call them:
  - Origin: "Origin Terminal" or "Pickup Service Center"
  - Destination: "Destination Terminal" or "Delivery Service Center"