async function updateForm() {
  const op = document.getElementById("operation").value;
  const container = document.getElementById("operation-form");
  const result = document.getElementById("result-message");
  result.textContent = "";

  if (!op) {
    container.innerHTML = "";
    return;
  }

  // Fetch available orders from backend
  let orders = [];
  try {
    const res = await fetch("/orders/list");
    if (res.ok) orders = await res.json();
  } catch {
    orders = [];
  }

  const orderOptions = orders
    .map(o => `<option value="${o.id}">#${o.id} - ${o.description || "Untitled"}</option>`)
    .join("");

  if (op === "scale") {
    container.innerHTML = `
      <div class="form-row">
        <label for="scaleOrder">Order:</label>
        <select id="scaleOrder" required>${orderOptions}</select>
      </div>
      <div class="form-row">
        <label for="scaleFactor">Scale Factor:</label>
        <input type="number" id="scaleFactor" step="0.1" value="1" required>
      </div>
      <button class="submit-button" onclick="submitScale()">Scale</button>
    `;
  } else if (op === "merge") {
    container.innerHTML = `
      <div class="form-row">
        <label for="mergeSource">Source Order:</label>
        <select id="mergeSource" required>${orderOptions}</select>
      </div>
      <div class="form-row">
        <label for="mergeTarget">Target Order:</label>
        <select id="mergeTarget" required>${orderOptions}</select>
      </div>
      <button class="submit-button" onclick="submitMerge()">Merge</button>
    `;
  } else if (op === "subtract") {
    container.innerHTML = `
      <div class="form-row">
        <label for="subFrom">From Order:</label>
        <select id="subFrom" required>${orderOptions}</select>
      </div>
      <div class="form-row">
        <label for="subWhat">Subtract Order:</label>
        <select id="subWhat" required>${orderOptions}</select>
      </div>
      <button class="submit-button" onclick="submitSubtract()">Subtract</button>
    `;
  }
}

async function submitScale() {
  const id = parseInt(document.getElementById("scaleOrder").value, 10);
  const factor = parseFloat(document.getElementById("scaleFactor").value);
  console.log("calling", id, factor);
  const res = await fetch("/orders/scale", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ order_id: id, scale_factor: factor })
  });
  showResult(res.ok, "Order scaled successfully!", "Error scaling order.");
}

async function submitMerge() {
  const source = parseInt(document.getElementById("mergeSource").value, 10);
  const target = parseInt(document.getElementById("mergeTarget").value, 10);
  const merge_options = 4; //document.getElementById("mergeOptions").value;
  if (source === target)
    return showResult(false, "", "Select two different orders.");
  console.log("calling", source, target);
  const res = await fetch("/orders/merge", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ source_id: source, target_id: target})
  });
  showResult(res.ok, "Orders merged successfully!", "Error merging orders.");
}

async function submitSubtract() {
  const from = document.getElementById("subFrom").value;
  const what = document.getElementById("subWhat").value;
  if (from === what)
    return showResult(false, "", "Select two different orders.");
  const res = await fetch("/orders/subtract", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ from_id: from, subtract_id: what })
  });
  showResult(res.ok, "Orders subtracted successfully!", "Error subtracting orders.");
}

function showResult(success, successMsg, errorMsg) {
  const el = document.getElementById("result-message");
  el.textContent = success ? `✅ ${successMsg}` : `❌ ${errorMsg}`;
  el.className = success ? "result-message success" : "result-message error";
}