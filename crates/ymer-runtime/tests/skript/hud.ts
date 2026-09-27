// Ett skript som bara rör gränssnittet.
//
// Det här är inte en illustration utan den fil testet faktiskt laddar:
// samma väg som ett spel tar, genom TypeScript-transformen, QuickJS i wasm
// och kommandobufferten tillbaka.

let klick = 0;

export function update(dt: number, entities: Entity[]): void {
  // Läsning: framens ögonblicksbild av dokumentet.
  if (ui.clicked("bygg")) {
    klick += 1;
    ui.setText("status", "byggt " + klick);
  }

  // Ett kryss ska stänga av knappen, inte bara logga.
  if (ui.changed("avancerat")) {
    ui.setVisible("bygg", ui.checked("avancerat"));
  }

  // Reglaget styr en etikett – siffran kommer från dokumentet, inte
  // från skriptets eget minne.
  if (ui.changed("volym")) {
    ui.setText("volym_text", "volym " + Math.round(ui.number("volym")));
  }

  // Enter i fältet flyttar texten till etiketten och nollar fältet.
  if (ui.submitted("namn")) {
    ui.setText("halsning", "hej " + ui.text("namn"));
    ui.setText("namn", "");
  }

  // Dropdown: null betyder inget val.
  if (ui.changed("svarighet")) {
    const index = ui.selected("svarighet");
    ui.setText("status", index === null ? "inget valt" : "grad " + index);
  }

  // Skriv tillbaka i den andra riktningen också, så att testet ser att
  // setValue och setSelected landar i dokumentet.
  if (ui.clicked("aterstall")) {
    ui.setValue("volym", 0);
    ui.setSelected("svarighet", null);
    ui.setChecked("avancerat", false);
  }
}
