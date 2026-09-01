# M0-6.c GUI Regression Evidence

STATUS=PASS
APP_EXIT=0
DRIVER_STATUS=PASS

## Checks

- PASS driver-status: driver status=PASS
- PASS scenario-1-event: single grid AI mock event with session cookie
- PASS scenario-2-grid-0: concurrent AI mock event for grid-0
- PASS scenario-2-grid-1: concurrent AI mock event for grid-1
- PASS scenario-2-grid-2: concurrent AI mock event for grid-2
- PASS scenario-2-grid-3: concurrent AI mock event for grid-3
- PASS scenario-3-event: main-window move/resize event reached grid-0
- PASS scenario-4-event: blur/focus recovery event reached grid-0
- PASS scenario-5-event: view-switch event reached grid-0
- PASS scenario-6-event: normal tab workflow left grid-0 usable
- PASS scenario-7-event: grid child crash recovery event reached restarted grid-0
- PASS scenario-8-menu: injected context menu opened inside grid webview
- PASS scenario-9-before: login-state cookie present before grid restart
- PASS scenario-9-after: login-state cookie present after grid restart

## Files

- app.log
- driver-report.json
- mock-events.jsonl
- summary.json
