CREATE OR REPLACE VIEW daily_view_fast_string as
select strftime(date, '%Y-%m-%d') as date, total_watt_hours 
FROM (SELECT * FROM current_view where date > (SELECT MAX(date) from materialized_daily)
      UNION ALL 
      SELECT * FROM materialized_daily) ORDER BY date DESC;
