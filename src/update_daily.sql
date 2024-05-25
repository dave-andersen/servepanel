CREATE OR REPLACE TABLE materialized_daily as select * from daily_view
WHERE date < current_date;