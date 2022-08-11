window.SIDEBAR_ITEMS = {"constant":[["DEFAULT_PORT",""]],"enum":[["ErrorKind","Potential categories an error from this library falls into."]],"mod":[["ext","Advanced extension points for the Cadence library"],["prelude","Export commonly used parts of Cadence for easy glob imports"]],"struct":[["BufferedSpyMetricSink","`MetricSink` implementation that buffers metrics and writes them to the `Sender` half of a channel while callers are given ownership of the `Receiver` half."],["BufferedUdpMetricSink","Implementation of a `MetricSink` that buffers metrics before sending them to a UDP socket."],["BufferedUnixMetricSink","Implementation of a `MetricSink` that buffers metrics before sending them to a Unix socket."],["Counter","Counters are simple values incremented or decremented by a client."],["Distribution","Distributions represent a global statistical distribution of a set of values."],["Gauge","Gauges are an instantaneous value determined by the client."],["Histogram","Histograms are values whose distribution is calculated by the server."],["Meter","Meters measure the rate at which events occur as determined by the server."],["MetricBuilder","Builder for adding tags to in-progress metrics."],["MetricError","Error generated by this library potentially wrapping another type of error (exposed via the `Error` trait)."],["NopMetricSink","Implementation of a `MetricSink` that discards all metrics."],["QueuingMetricSink","Implementation of a `MetricSink` that wraps another implementation and uses it to emit metrics asynchronously, in another thread."],["Set","Sets count the number of unique elements in a group."],["SpyMetricSink","`MetricSink` implementation that writes all metrics to the `Sender` half of a channel while callers are given ownership of the `Receiver` half."],["StatsdClient","Client for Statsd that implements various traits to record metrics."],["StatsdClientBuilder","Builder for creating and customizing `StatsdClient` instances."],["Timer","Timers are a positive number of milliseconds between a start and end point."],["UdpMetricSink","Implementation of a `MetricSink` that emits metrics over UDP."],["UnixMetricSink","Implementation of a `MetricSink` that emits metrics over a Unix socket."]],"trait":[["Compat","Backwards compatibility shim for removed and deprecated methods."],["Counted","Trait for incrementing and decrementing counters."],["CountedExt","Trait for convenience methods for counters"],["Distributed","Trait for recording distribution values."],["Gauged","Trait for recording gauge values."],["Histogrammed","Trait for recording histogram values."],["Metered","Trait for recording meter values."],["Metric","Trait for metrics to expose Statsd metric string slice representation."],["MetricClient","Trait that encompasses all other traits for sending metrics."],["MetricSink","Trait for various backends that send Statsd metrics somewhere."],["Setted","Trait for recording set values."],["Timed","Trait for recording timings in milliseconds."]],"type":[["MetricResult",""]]};