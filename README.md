Serve solar power production data from a local duckdb database.

Not really packaged for external use yet; just part of my toy solar
experiment. Data is logged into a local JSON file; a cron job
moves the JSON data into duckdb nightly. The interface unions
across both the duck instance and the inbound JSON log to
serve queries.

Data is logged by a separate tiny python process that receives
UDP packets from the wattmeter attached to the solar input.
