// Client-side search and filters for the order tables (see order_filters.html).
// Note: filters the rows already on the page; move to query parameters and
// SQL if the lists get long enough to need pagination.
document.addEventListener('DOMContentLoaded', () => {
    const rows = Array.from(document.querySelectorAll('.orders-table tbody tr'));
    const search = document.getElementById('order-search');
    const selects = Array.from(document.querySelectorAll('.order-filter'));
    const count = document.getElementById('order-count');
    if (!search) return;

    // Each select offers the values present in the table.
    selects.forEach((select) => {
        const values = [...new Set(rows.map((row) => row.dataset[select.dataset.key]))].sort();
        values.forEach((value) => select.add(new Option(value, value)));
    });

    // Text of the data cells only, so button labels ("Edit") do not match a search.
    const texts = rows.map((row) =>
        Array.from(row.cells)
            .filter((cell) => !cell.classList.contains('actions-cell'))
            .map((cell) => cell.textContent)
            .join(' ')
            .toLowerCase()
    );

    const apply = () => {
        const query = search.value.trim().toLowerCase();
        let shown = 0;
        rows.forEach((row, i) => {
            const visible = texts[i].includes(query)
                && selects.every((select) => !select.value || row.dataset[select.dataset.key] === select.value);
            row.hidden = !visible;
            if (visible) shown++;
        });
        count.textContent = `${shown} of ${rows.length} orders`;
    };

    search.addEventListener('input', apply);
    selects.forEach((select) => select.addEventListener('change', apply));
    apply();
});
