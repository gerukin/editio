-- Aggregation and a string literal.
SELECT team, COUNT(*) AS readers
FROM people
WHERE active = TRUE AND name <> 'anonymous'
GROUP BY team
HAVING COUNT(*) > 2
ORDER BY readers DESC;
