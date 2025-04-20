let itemIndex = 0;

function addItemEntry(proposal = "", project = "", manifacturer = "", manifacturerPn = "", quantity = 1) {
    const container = document.getElementById('items-container');

    const div = document.createElement('div');
    div.className = 'item-entry';
    div.innerHTML = `
        <label>Proposal:</label>
        <input type="text" name="items_proposal_${itemIndex}" value="${proposal}" required />
        <label>Project:</label>
        <input type="text" name="items_project_${itemIndex}" value="${project}" required />
        
        <label>Manifacturer:</label>
        <input type="text" name="items_manifacturer_${itemIndex}" value="${manifacturer}" required />
        <label>P.N.:</label>
        <input type="text" name="items_manifacturer_pn_${itemIndex}" value="${manifacturerPn}" required />
        <label>Quantity:</label>
        <input type="number" name="items_quantity_${itemIndex}" value="${quantity}" required />
        <button type="button" class="remove-button" onclick="removeItemEntry(this)">×</button>
    `;
    container.appendChild(div);
    itemIndex++;
}

function removeItemEntry(button) {
    const entry = button.parentElement;
    entry.remove();
}
