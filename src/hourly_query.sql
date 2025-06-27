select strftime(hour, '%Y-%m-%d %H'), 
round(sum(power * (CASE when epoch(dur) > 60 THEN 60 ELSE epoch(dur) END))/(60*60)) as total_watt_hours 
FROM (
   SELECT power,
          date_trunc('hour', time at time zone 'EST') as hour,
          time - lag(time) over (ORDER BY TIME) as dur 
   FROM (SELECT * from (SELECT * from powerlog UNION ALL 
         SELECT * from read_json_auto('/home/dga/solar/log.json', format='newline_delimited',
                                             columns = {time: 'timestamptz', power: 'int32'}))
	 WHERE time >= (time_bucket(INTERVAL 1 DAY, now() at time zone 'EST') - INTERVAL 1 DAY) at time zone 'EST')
) where dur is not null and power >= 4 GROUP BY hour ORDER BY hour;
