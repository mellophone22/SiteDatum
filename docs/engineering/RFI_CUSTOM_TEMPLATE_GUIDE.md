# RFI custom PDF template guide

Custom RFI templates are a SiteDatum Pro capability. A template remains a normal customer-owned Windows file; SiteDatum stores its path and reads it only when creating an RFI PDF.

## File requirements

- PDF format, unencrypted.
- Exactly one page.
- Portrait US Letter: 612 × 792 PDF points, with a tolerance of 3 points.
- Permanent logos, company details, borders, and labels should be placed in the template itself.
- Keep the field regions below free of important artwork. Coordinates use PDF points measured from the lower-left corner.

## Field-position contract

| Field | Left | Baseline | Available width | Maximum lines |
| --- | ---: | ---: | ---: | ---: |
| RFI number | 414 | 698 | 140 | 1 |
| Date | 414 | 680 | 140 | 1 |
| Project name | 414 | 662 | 140 | 2 |
| Recipient | 68 | 602 | 218 | 3 |
| Project location | 343 | 602 | 216 | 3 |
| Cost impact | 132 | 532 | 154 | 1 |
| Time delay | 132 | 514 | 154 | 1 |
| RFI location | 414 | 532 | 144 | 1 |
| Drawing number | 414 | 514 | 144 | 1 |
| Subject | 68 | 472 | 490 | 2 |
| Question | 68 | 448 | 490 | 13 |
| Suggested solution | 68 | 286 | 490 | 13 |
| Requested by | 414 | 87 | 145 | 1 |

SiteDatum uses Helvetica and Helvetica Bold for overlay text. Content that cannot fit is rejected before the destination is finalized; the user can shorten the RFI or attach supporting detail. Existing files are never overwritten.

## Privacy and recovery

Template artwork and RFI content are processed locally. They are never sent to licensing, billing, synchronization, or telemetry services. If a template resides on a disconnected drive, reconnect it or switch back to the SiteDatum layout in Settings. Removing a template setting does not alter the source PDF or previously generated RFIs.
