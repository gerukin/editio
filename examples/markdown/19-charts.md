# XY charts: series, axes, orientation, and ranges

## Mixed bars and line series

```mermaid
xychart-beta
    title "Documents created and reviewed"
    x-axis "Week" [Mon, Tue, Wed, Thu, Fri]
    y-axis "Documents" 0 --> 20
    bar [4, 8, 12, 7, 16]
    line [3, 6, 10, 11, 15]
```

## Horizontal categories

```mermaid
xychart-beta horizontal
    title "Files by category"
    x-axis [Notes, Articles, Tasks, Drafts]
    y-axis "Count" 0 --> 30
    bar [24, 18, 12, 6]
```

## Numeric axis and negative values

```mermaid
xychart-beta
    title "Net change"
    x-axis "Day" 1 --> 5
    y-axis "Change" -10 --> 10
    line [-6, -2, 5, 2, 8]
```

## Multiple lines with inferred x labels

```mermaid
xychart-beta
    title "Processing time"
    y-axis "Milliseconds" 0 --> 100
    line [80, 60, 45, 30]
    line [90, 75, 65, 50]
```
