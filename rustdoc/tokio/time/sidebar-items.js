initSidebarItems({"enum":[["MissedTickBehavior","Defines the behavior of an [`Interval`] when it misses a tick."]],"fn":[["advance","Advances time."],["interval","Creates new [`Interval`] that yields with interval of `period`. The first tick completes immediately. The default [`MissedTickBehavior`] is `Burst`, but this can be configured by calling `set_missed_tick_behavior`."],["interval_at","Creates new [`Interval`] that yields with interval of `period` with the first tick completing at `start`. The default [`MissedTickBehavior`] is `Burst`, but this can be configured by calling `set_missed_tick_behavior`."],["pause","Pauses time."],["resume","Resumes time."],["sleep","Waits until `duration` has elapsed."],["sleep_until","Waits until `deadline` is reached."],["timeout","Requires a `Future` to complete before the specified duration has elapsed."],["timeout_at","Requires a `Future` to complete before the specified instant in time."]],"mod":[["error","Time error types."]],"struct":[["Instant","A measurement of a monotonically nondecreasing clock. Opaque and useful only with `Duration`."],["Interval","Interval returned by [`interval`] and [`interval_at`]."],["Sleep","Future returned by `sleep` and `sleep_until`."],["Timeout","Future returned by `timeout` and `timeout_at`."]]});