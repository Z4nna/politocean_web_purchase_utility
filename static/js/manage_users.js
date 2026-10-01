// Constrains every sub-area dropdown to sub-areas that form a valid,
// non-archived pair with the division selected next to it (same table row, or
// the create form). A user's current pair stays selectable.
document.addEventListener('DOMContentLoaded', () => {
    const pairs = Array.from(document.getElementById('area-pairs-data').options)
        .filter((o) => o.dataset.archived !== "true")
        .map((o) => ({ division: o.dataset.division, sub_area: o.value }));

    document.querySelectorAll('select[name="belonging_area_division"]').forEach((divisionSelect) => {
        const subAreaSelect = divisionSelect
            .closest('tr, form')
            .querySelector('select[name="belonging_area_sub_area"]');
        const originalDivision = divisionSelect.value;
        const originalSubArea = subAreaSelect.value;

        const restrict = () => {
            const division = divisionSelect.value;
            const valid = new Set(pairs.filter((p) => p.division === division).map((p) => p.sub_area));
            if (division === originalDivision) valid.add(originalSubArea);

            Array.from(subAreaSelect.options).forEach((opt) => {
                opt.disabled = opt.hidden = !valid.has(opt.value);
            });
            // If the current sub-area is not valid for the new division, move to the first valid one.
            if (subAreaSelect.selectedOptions[0]?.disabled !== false) {
                const firstOk = Array.from(subAreaSelect.options).find((o) => !o.disabled);
                subAreaSelect.value = firstOk ? firstOk.value : "";
            }
        };

        divisionSelect.addEventListener('change', restrict);
        restrict();
    });
});
