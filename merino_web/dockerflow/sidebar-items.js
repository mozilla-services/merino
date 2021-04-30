initSidebarItems({"enum":[["CheckStatus","The status of an individual check, or the whole system, as reported by /heartbeat."]],"fn":[["service","Handles required Dockerflow Endpoints."]],"struct":[["HeartbeatResponse","A response to the `/__heartbeat__` endpoint."],["heartbeat","Returns a status message indicating the current state of the server."],["lbheartbeat","Used by the load balancer to indicate that the server can respond to requests. Should just return OK."],["test_error","Returning an API error to test error handling."],["version","Return the contents of the `version.json` file created by CircleCI and stored in the Docker root (or the TBD version stored in the Git repo)."]]});