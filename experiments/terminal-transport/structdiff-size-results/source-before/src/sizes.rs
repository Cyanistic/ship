//! Larger native captures, deliberately a different workload from scroll-output's 12-line burst.
use super::{bench, run_native, settled, text};
use ratatui::style::{Color, Modifier};
use std::path::Path;

const GRID: &[(u16, u16, &str)] = &[
    (80, 24, "small"),
    (120, 40, "medium"),
    (160, 50, "full-screen-example"),
    (80, 50, "vertical-half"),
    (160, 25, "horizontal-half"),
];

pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!cfg!(debug_assertions), "run this benchmark with --release");
    for &(cols, rows, label) in GRID {
        let output = Path::new("size-results").join(format!("{cols}x{rows}"));
        std::fs::create_dir_all(&output)?;
        // Fill every native row, including its final column, with varied ASCII and RGB.
        // CUP cancels pending wrap. Only the explicit bottom-row CR LF scrolls.
        let script = format!(
            r#"stty -echo
cols={cols}; rows={rows}
printf '\033[2J\033[H'
fill_row() {{
    line=$(printf '%03d:' "$r")
    j=4
    while [ "$j" -lt "$cols" ]; do
        line="$line$(((r+j)%10))"
        j=$((j+1))
    done
    printf '\033[%d;1H\033[38;2;%d;20;30;48;2;10;%d;50m%s\033[0m' "$y" "$((r%200+30))" "$((r%150+50))" "$line"
}}
r=1
while [ "$r" -le "$rows" ]; do
    y=$r; fill_row
    r=$((r+1))
done
printf '\033[1;10H\033[1;38;2;220;30;40;48;2;10;60;100mcafé 界 é\033[0m'
printf '\033[%d;1HSIZE-READY\033[%d;1H' "$rows" "$rows"
read x
printf '\033[2;1H\033[3;4;38;2;10;20;30mEDIT\033[0m\033[%d;1H' "$rows"
read x
printf '\033[%d;1H\r\n' "$rows"
r=$((rows+1)); y=$rows; fill_row
printf '\033[%d;1HONE-SCROLL\033[%d;1H' "$rows" "$rows"
read x
exit 0
"#
        );
        std::fs::write(output.join("workload.sh"), &script)?;
        println!(
            "SIZE START {cols}x{rows} label={label} workload=filled-native-screen+four-cell-text-style-edit+one-row-scroll+unchanged chosen_dimensions=true"
        );
        let captures = run_native(cols, rows, &script, |session| {
            let initial = settled(session, "SIZE-READY", (cols, rows));
            assert!(text(&initial).contains("café 界") && text(&initial).contains("é"));
            assert!(
                initial
                    .cells
                    .iter()
                    .any(|c| c.modifiers.contains(Modifier::BOLD))
            );
            assert!(initial.cells.iter().any(
                |c| matches!(c.fg, Color::Rgb(_, _, _)) && matches!(c.bg, Color::Rgb(_, _, _))
            ));
            for row in initial.cells.chunks(cols as usize) {
                assert!(
                    row.iter().filter(|c| !c.text.trim().is_empty()).count() > cols as usize / 2,
                    "native row not filled"
                );
                assert!(
                    row.windows(2).any(|p| p[0].text != p[1].text),
                    "row not varied"
                );
            }
            assert_eq!(initial.cursor.position, Some((0, rows - 1)));
            session.send_raw(b"\n".to_vec());
            let edited = settled(session, "EDIT", (cols, rows));
            let changed: Vec<_> = initial
                .cells
                .iter()
                .zip(&edited.cells)
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(
                changed,
                (cols as usize..cols as usize + 4).collect::<Vec<_>>()
            );
            assert_eq!(initial.cursor, edited.cursor);
            assert!(
                edited.cells[cols as usize..cols as usize + 4]
                    .iter()
                    .all(|c| c.fg == Color::Rgb(10, 20, 30)
                        && c.modifiers.contains(Modifier::UNDERLINED)
                        && c.modifiers.contains(Modifier::ITALIC))
            );
            session.send_raw(b"\n".to_vec());
            let scrolled = settled(session, "ONE-SCROLL", (cols, rows));
            let retained = (rows as usize - 1) * cols as usize;
            assert_eq!(
                scrolled.cells[..retained],
                edited.cells[cols as usize..],
                "not exactly a one-row native scroll"
            );
            assert_ne!(scrolled.cells[retained..], edited.cells[retained..]);
            assert_eq!(scrolled.cursor, edited.cursor);
            let unchanged = settled(session, "ONE-SCROLL", (cols, rows));
            assert_eq!(unchanged, scrolled);
            session.send_raw(b"\n".to_vec());
            println!(
                "SIZE WORKFLOW PASS {cols}x{rows}: every row filled/varied, Unicode/bold/RGB preserved, exactly four edited cells + underline/italic, bottom reached, exact one-row shift, unchanged identical"
            );
            vec![
                ("initial-filled", initial),
                ("small-edit", edited),
                ("one-row-scroll", scrolled),
                ("unchanged", unchanged),
            ]
        })?;
        bench::run_quick(&captures, &output);
    }
    println!(
        "SIZE GRID PASS: five native shells reaped, readers dropped, sessions stopped; 90 metrics with 20 warmups + 100 samples each; no structdiff comparator measured"
    );
    Ok(())
}
