CREATE OR REPLACE VIEW daily_view_fast as
select date, total_watt_hours 
FROM (SELECT * FROM current_view where date > (SELECT MAX(date) from materialized_daily)
      UNION ALL 
      SELECT * FROM materialized_daily) ORDER BY date DESC;

CREATE OR REPLACE VIEW daily_view_fast_string as
select strftime(date, '%Y-%m-%d') as date, total_watt_hours
FROM daily_view_fast;

CREATE OR REPLACE VIEW monthly_fast AS
select date_trunc('month', date) as month, sum(total_watt_hours) 
from daily_view_fast GROUP BY month ORDER BY month;