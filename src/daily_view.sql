CREATE OR REPLACE VIEW daily_view as SELECT date, round(sum(power * dur)/(60*60)) as total_watt_hours
FROM (SELECT power, date_trunc('day', time at time zone 'EST') as date, 
            epoch(time - lag(time) over (ORDER BY TIME)) as dur_internal,
            case when dur_internal > 60 then 60 else dur_internal END as dur 
      from (SELECT * from powerlog UNION ALL 
            SELECT * from read_json_auto('/home/dga/solar/log.json', format='newline_delimited',
                 columns = {time: 'timestamptz', power: 'int32'})))
WHERE dur is not null and power >= 4
GROUP BY date
ORDER BY date DESC;

CREATE OR REPLACE VIEW current_view as SELECT date, round(sum(power * dur)/(60*60)) as total_watt_hours
FROM (SELECT power, date_trunc('day', time at time zone 'EST') as date, 
            epoch(time - lag(time) over (ORDER BY TIME)) as dur_internal,
            case when dur_internal > 60 then 60 else dur_internal END as dur 
      from (
            SELECT * from read_json_auto('/home/dga/solar/log.json', format='newline_delimited',
                 columns = {time: 'timestamptz', power: 'int32'})))
WHERE dur is not null and power >= 4
GROUP BY date
ORDER BY date DESC;