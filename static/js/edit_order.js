function showLoadingContainer() {
    document.getElementById('loading-container').style.display = 'flex';
}

// The order's area at page load. Kept selectable even if that pair is archived.
let originalDivision = null;
let originalSubArea = null;

// Reads the valid (division, sub_area) pairs embedded in #area-pairs-data.
function getAreaPairs() {
    const holder = document.getElementById('area-pairs-data');
    if (!holder) return [];
    return Array.from(holder.options).map((o) => ({
        division: o.dataset.division,
        sub_area: o.value,
        archived: o.dataset.archived === "true",
    }));
}

// Constrains the sub-area dropdown to sub-areas that form a valid, non-archived
// pair with the selected division, so an edit can never build an invalid
// (division, sub_area) composite key. The order's original pair stays selectable.
function restrictSubAreas() {
    const divisionSelect = document.getElementById('area_division');
    const subAreaSelect = document.getElementById('area_sub_area');
    if (!divisionSelect || !subAreaSelect) return;

    const division = divisionSelect.value;
    const valid = new Set(
        getAreaPairs().filter((p) => p.division === division && !p.archived).map((p) => p.sub_area)
    );
    const keepOriginal = division === originalDivision ? originalSubArea : null;

    let selectedStillValid = false;
    Array.from(subAreaSelect.options).forEach((opt) => {
        const ok = valid.has(opt.value) || opt.value === keepOriginal;
        opt.disabled = !ok;
        if (opt.selected && ok) selectedStillValid = true;
    });
    // If the current sub-area is not valid for the new division, move to the first valid one.
    if (!selectedStillValid) {
        const firstOk = Array.from(subAreaSelect.options).find((o) => !o.disabled);
        subAreaSelect.value = firstOk ? firstOk.value : "";
    }
}

document.addEventListener('DOMContentLoaded', () => {
    const divisionSelect = document.getElementById('area_division');
    const subAreaSelect = document.getElementById('area_sub_area');
    if (divisionSelect && subAreaSelect) {
        originalDivision = divisionSelect.value;
        originalSubArea = subAreaSelect.value;
        restrictSubAreas();
    }
});

let itemIndex = 0;

// Disables archived options (data-archived="true") that are not the current
// selection, so an item keeps and submits its own (possibly archived) value
// while archived options cannot be chosen for new or other items.
function lockArchivedOptions(select) {
    Array.from(select.options).forEach((opt) => {
        if (opt.dataset.archived === "true" && !opt.selected) {
            opt.disabled = true;
        }
    });
}

function addItemEntry(proposal = "", project = "", manufacturer = "", manufacturerPn = "", quantity = 1) {
    const container = document.getElementById('items-container');

    const div = document.createElement('div');
    div.className = 'item-entry';

    const proposalSelect = document.createElement('select');
    proposalSelect.name = `items_proposal_${itemIndex}`;
    proposalSelect.required = true;
    proposalSelect.innerHTML = document.getElementById('proposal-template').innerHTML;
    // Select the item's value only after the options exist. Setting it before
    // (or for an empty new row) would leave the select on a random first option.
    if (proposal) proposalSelect.value = proposal;
    lockArchivedOptions(proposalSelect);

    const projectSelect = document.createElement('select');
    projectSelect.name = `items_project_${itemIndex}`;
    projectSelect.required = true;
    projectSelect.innerHTML = document.getElementById('project-template').innerHTML;
    if (project) projectSelect.value = project;
    lockArchivedOptions(projectSelect);

    const manufacturerInput = document.createElement('input');
    manufacturerInput.type = 'text';
    manufacturerInput.value = manufacturer;
    manufacturerInput.name = `items_manufacturer_${itemIndex}`;
    manufacturerInput.required = true;

    const manufacturerPnInput = document.createElement('input');
    manufacturerPnInput.type = 'text';
    manufacturerPnInput.value = manufacturerPn;
    manufacturerPnInput.name = `items_manufacturer_pn_${itemIndex}`;
    manufacturerPnInput.required = true;

    const quantityInput = document.createElement('input');
    quantityInput.type = 'number';
    quantityInput.name = `items_quantity_${itemIndex}`;
    quantityInput.value = quantity;
    quantityInput.required = true;

    const deleteButton = document.createElement('button');
    deleteButton.type = 'button';
    deleteButton.className = 'delete-button';
    deleteButton.textContent = '×';
    deleteButton.onclick = () => removeItemEntry(deleteButton);

    div.appendChild(proposalSelect);
    div.appendChild(projectSelect);
    div.appendChild(manufacturerInput);
    div.appendChild(manufacturerPnInput);
    div.appendChild(quantityInput);
    div.appendChild(deleteButton);

    container.appendChild(div);
    itemIndex++;
}

function removeItemEntry(button) {
    const entry = button.parentElement;
    entry.remove();
}

