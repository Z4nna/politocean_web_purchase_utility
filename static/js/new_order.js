let itemIndex = 0;

function addItemEntry() {
    const container = document.getElementById('items-container');

    const div = document.createElement('div');
    div.className = 'item-entry';
    div.innerHTML = `
        <input type="text" name="items_proposal_${itemIndex}" value = "Sensoristica AUV" required />
        <input type="text" name="items_project_${itemIndex}" value = "Nereo" required />
        
        <input type="text" name="items_manifacturer_${itemIndex}" required />
        <input type="text" name="items_manifacturer_pn_${itemIndex}" required />
        <input type="number" name="items_quantity_${itemIndex}" value = "1" required />
        <button type="button" class="remove-button" onclick="removeItemEntry(this)">×</button>
    `;
    container.appendChild(div);
    itemIndex++;
}

function removeItemEntry(button) {
    const entry = button.parentElement;
    entry.remove();
}

window.onload = function () {
    addItemEntry(); // Add one by default
};