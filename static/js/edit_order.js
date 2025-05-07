let itemIndex = 0;

function showLoadingContainer() {
    document.getElementById('loading-container').style.display = 'flex';
}

function addItemEntry(proposal = "", project = "", manifacturer = "", manifacturerPn = "", quantity = 1) {
    const container = document.getElementById('items-container');

    const div = document.createElement('div');
    div.className = 'item-entry';
    div.innerHTML = `
        <input type="text" name="items_proposal_${itemIndex}" value="${proposal}" required />
        <input type="text" name="items_project_${itemIndex}" value="${project}" required />
        
        <input type="text" name="items_manifacturer_${itemIndex}" value="${manifacturer}" required />
        <input type="text" name="items_manifacturer_pn_${itemIndex}" value="${manifacturerPn}" required />
        <input type="number" name="items_quantity_${itemIndex}" value="${quantity}" required />
        <button type="button" class="delete-button" onclick="removeItemEntry(this)">×</button>
    `;
    container.appendChild(div);
    itemIndex++;
}

function removeItemEntry(button) {
    const entry = button.parentElement;
    entry.remove();
}

