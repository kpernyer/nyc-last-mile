# Analytics / Curiosity

## 🚚 1. Descriptive Analytics (What is happening?)

Basic performance KPIs
	•	Overall OTD rate (already have)
	•	OTD by carrier
	•	OTD by lane (origin→ZIP3)
	•	OTD by customer / delivery region
	•	OTD by mode (LTL, TL, etc.)
	•	OTD by shipment size / weight
	•	OTD by day of week / pickup day

Transit time performance
	•	Actual vs Goal days by distance segment (already started)
	•	Transit time distributions (histograms)
	•	Variance / standard deviation by carrier and lane
	•	Percent of “very late” shipments (e.g., >2 days late)

Volume analytics
	•	Shipments by carrier
	•	Shipments by lane
	•	Shipments by ZIP3
	•	Seasonal / monthly trends


## 🔍 2. Diagnostic Analytics (Why is it happening?)

Root-cause breakdowns
	•	Late vs Early root causes:
	•	Pickup delay?
	•	Hub processing delay?
	•	Line haul capacity issues?
	•	Weather spikes?
	•	Carrier underperformance?

Carrier performance comparisons
	•	Benchmark each carrier:
	•	OTD %
	•	Early/Late distribution shape
	•	Avg days above goal
	•	Variability
	•	Failure rate on long-haul vs short-haul

Lane diagnostics
	•	Identify worst-performing lanes using:
	•	OTD delta vs network average
	•	Delay clustering
	•	Bottleneck locations (terminal, crossdock)

ZIP3 problem hotspots
	•	Which ZIP3 areas systematically fail?
	•	Early deliveries clustered around large metros? (usually yes)
	•	Rural ZIP3s with predictable delay patterns?

Mode efficiency
	•	Compare LTL vs TL actual performance across distance bands
	•	Assess whether more TL runs reduce variability


## 🔮 3. Predictive Analytics (What will happen?)

ETA prediction models
	•	Predict delivery date at pickup time using:
	•	Carrier historical performance
	•	Lane and ZIP3
	•	Weather
	•	Terminal congestion signals
	•	Seasonality

Delay likelihood scoring
	•	Probability of being Late/Early given:
	•	Day of week
	•	Distance
	•	Carrier
	•	Lane congestion history

Capacity forecasting
	•	Predict volume surges
	•	Recommend carrier reallocation before performance drops


## 🛠️ 4. Prescriptive Analytics (What should we do?)

Carrier optimization
	•	Recommend shifting volume to best carriers on worst lanes
	•	Suggest lane-level carrier blends that maximize OTD

Routing & mode optimization
	•	For lanes where LTL consistently fails:
	•	Simulate TL conversion impact on OTD
	•	Cost vs performance tradeoff

Dynamic SLA setting
	•	Create data-based SLA goals per distance band
	•	Propose realistic SLAs by carrier and lane
	•	Adjust customer promise dates

Exception management
	•	Trigger alerts when a lane or carrier suddenly deviates from baseline
	•	Tiered escalation rules based on predicted delay severity


## 📊 5. Advanced Analytics (Deep insights & operational levers)

1. Early deliveries — hidden cost analysis

Early deliveries look good on paper, but:
	•	Increase cost via misaligned labor at destination
	•	Trigger warehouse congestion
	•	Cause poor pickup planning

Quantify cost impact vs benefit.

2. Lane clustering

Cluster lanes by:
	•	Similar transit profiles
	•	Similar variability
	•	Similar carrier performance

Used to simplify network planning.

3. Geo-spatial analysis
	•	Map delivery times across ZIP3 regions
	•	Identify geographical friction zones
	•	Detect network imbalance

4. Carrier terminal performance index

Evaluate each terminal’s contribution to:
	•	Delay variance
	•	Routing inefficiencies
	•	Congestion patterns


🧭 What this enables

With the dataset you have, the goal should be:

→ Build a complete situational awareness layer for last-mile performance,
→ Build prediction models,
→ Build optimization recommendations.
