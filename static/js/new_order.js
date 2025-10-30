let itemIndex = 0;

function addItemEntry(proposal = "", project = "", manufacturer = "", manufacturerPn = "", quantity = 1) {
    const container = document.getElementById('items-container');

    const div = document.createElement('div');
    div.className = 'item-entry';

    const proposalSelect = document.createElement('select');
    proposalSelect.name = `items_proposal_${itemIndex}`;
    proposalSelect.value = proposal;
    proposalSelect.required = true;
    proposalSelect.innerHTML = document.getElementById('proposal-template').innerHTML;

    const projectSelect = document.createElement('select');
    projectSelect.name = `items_project_${itemIndex}`;
    projectSelect.value = project;
    projectSelect.required = true;
    projectSelect.innerHTML = document.getElementById('project-template').innerHTML;

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

window.onload = function () {
    addItemEntry(); // Add one by default
};