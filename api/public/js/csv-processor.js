/**
 * @type {string}
 * @description ID of the uploaded file.
 */
let uploadedFileId = "";

/**
 * @type {Array<string>}
 * @description Array to store the columns of the uploaded CSV file.
 */
let csvColumns = [];

/**
 * @type {number | null}
 * @description Interval ID for tracking the current progress, or null if not set.
 */
let currentProgressInterval = null;

/**
 * @type {number | null}
 * @description Interval ID for the timer, or null if not set.
 */
let timerInterval = null;

/**
 * @type {Date | null}
 * @description Start time of the process, or null if not set.
 */
let startTime = null;

/**
 * @type {Object<string, string>}
 * @description Object to map fields from the CSV to their corresponding values.
 */
let fieldMapping = {};

/**
 * @type {Object<string, string>}
 * @description Object to map attributes from the CSV to their corresponding values.
 */
let attributeMappings = {};

/**
 * @type {number}
 * @description Counter to track the number of attributes processed.
 */
let attributeCounter = 0;

/**
 * @type {Array<Object>}
 * @description Array to store site configurations
 */
let sitesList = [];

/**
 * @type {string|null}
 * @description Currently selected site ID
 */
let currentSiteId = null;

// WooCommerce fields from the provided HTML
const woocommerceFields = [
  {
    value: "id",
    label: "ID",
    required: true,
    description: "Unique product ID (auto-generated if empty)",
  },
  {
    value: "type",
    label: "Type",
    required: true,
    description: "Product type (simple, variable, grouped, external)",
  },
  {
    value: "sku",
    label: "SKU",
    required: true,
    description: "Unique SKU of product",
  },
  {
    value: "parent_id",
    label: "Parent",
    required: true,
    description: "Parent product SKU (for variations)",
  },
  {
    value: "global_unique_id",
    label: "GTIN, UPC, EAN, or ISBN",
    required: false,
    description: "Global product identifier",
  },
  {
    value: "name",
    label: "Name",
    required: true,
    description: "Product name/title",
  },
  {
    value: "published",
    label: "Published",
    required: false,
    description: "Product visibility status (yes/no)",
  },
  {
    value: "featured",
    label: "Is featured?",
    required: false,
    description: "Display product as featured (yes/no)",
  },
  {
    value: "catalog_visibility",
    label: "Visibility in catalog",
    required: false,
    description:
      "Controls where product appears (visible, catalog, search, hidden)",
  },
  {
    value: "description",
    label: "Description",
    required: false,
    description: "Full product description",
  },
  {
    value: "short_description",
    label: "Short description",
    required: false,
    description: "Brief product summary shown in listings",
  },
  {
    value: "regular_price",
    label: "Regular price",
    required: false,
    description: "Main product price",
  },
  {
    value: "sale_price",
    label: "Sale price",
    required: false,
    description: "Discounted price (if on sale)",
  },
  {
    value: "date_on_sale_from",
    label: "Date sale price starts",
    required: false,
    description: "Sale start date (YYYY-MM-DD)",
  },
  {
    value: "date_on_sale_to",
    label: "Date sale price ends",
    required: false,
    description: "Sale end date (YYYY-MM-DD)",
  },
  {
    value: "tax_status",
    label: "Tax status",
    required: false,
    description: "Product tax status (taxable, shipping, none)",
  },
  {
    value: "tax_class",
    label: "Tax class",
    required: false,
    description: "Tax classification",
  },
  {
    value: "stock_status",
    label: "In stock?",
    required: false,
    description: "Stock status (instock, outofstock, onbackorder)",
  },
  {
    value: "stock_quantity",
    label: "Stock",
    required: false,
    description: "Available inventory quantity",
  },
  {
    value: "manage_stock",
    label: "Manage Stock",
    required: false,
    description: "If to manage stock (true, yes or 1)",
  },
  {
    value: "backorders",
    label: "Backorders allowed?",
    required: false,
    description: "Allow backorders (yes, no, notify)",
  },
  {
    value: "low_stock_amount",
    label: "Low stock amount",
    required: false,
    description: "Threshold for low stock alerts",
  },
  {
    value: "sold_individually",
    label: "Sold individually?",
    required: false,
    description: "Limit to one per order (yes/no)",
  },
  {
    value: "weight",
    label: "Weight (kg)",
    required: false,
    description: "Product weight in kilograms",
  },
  {
    value: "length",
    label: "Length (cm)",
    required: false,
    description: "Product length in centimeters",
  },
  {
    value: "width",
    label: "Width (cm)",
    required: false,
    description: "Product width in centimeters",
  },
  {
    value: "height",
    label: "Height (cm)",
    required: false,
    description: "Product height in centimeters",
  },
  {
    value: "category_ids",
    label: "Categories",
    required: false,
    description: "Product categories (separated IDs by |)",
  },
  {
    value: "tag_ids",
    label: "Tags (| separated)",
    required: false,
    description: "Product tags separated by |",
  },
  {
    value: "shipping_class_id",
    label: "Shipping class",
    required: false,
    description: "Shipping class ID or name",
  },
  {
    value: "images",
    label: "Images",
    required: false,
    description: "Product images (URLs, separated by pipe |)",
  },
  {
    value: "featured_image",
    label: "Featured Image",
    required: false,
    description: "Product Featured Image (URL)",
  },
  {
    value: "upsell_ids",
    label: "Upsells",
    required: false,
    description: "Upsell product IDs (comma separated)",
  },
  {
    value: "cross_sell_ids",
    label: "Cross-sells",
    required: false,
    description: "Cross-sell product IDs (comma separated)",
  },
  {
    value: "grouped_products",
    label: "Grouped products",
    required: false,
    description: "Grouped product IDs (comma separated)",
  },
  {
    value: "product_url",
    label: "External URL",
    required: false,
    description: "URL for external products",
  },
  {
    value: "button_text",
    label: "Button text",
    required: false,
    description: "Button text for external products",
  },
  {
    value: "download_limit",
    label: "Download limit",
    required: false,
    description: "Download limit for downloadable products",
  },
  {
    value: "download_expiry",
    label: "Download expiry days",
    required: false,
    description: "Days until download expires",
  },
  {
    value: "reviews_allowed",
    label: "Allow customer reviews?",
    required: false,
    description: "Enable customer reviews (yes/no)",
  },
  {
    value: "purchase_note",
    label: "Purchase note",
    required: false,
    description: "Note shown after purchase",
  },
  {
    value: "menu_order",
    label: "Position",
    required: false,
    description: "Product sorting position",
  },
];

// LocalStorage keys
const FIELD_MAPPING_KEY = "csvProcessor_fieldMapping";
const ATTRIBUTE_MAPPING_KEY = "csvProcessor_attributeMapping";
const IS_NEW_UPLOAD_KEY = "csvProcessor_isNewUpload";
const SITES_LIST_KEY = "csvProcessor_sites";
const CURRENT_SITE_KEY = "csvProcessor_currentSite";

// Site management functions

/**
 * Initializes the site management and loads saved sites
 */
/**
 * Initializes the site management and loads saved sites
 */
function initSites() {
  loadSitesFromStorage();
  populateSiteSelector();

  // Set up event listener for site selection
  $("#site-select").on("change", function () {
    const selectedSiteId = $(this).val();
    if (selectedSiteId) {
      selectSite(selectedSiteId);
    }
  });

  // Add DOM ready event handler
  $(document).ready(function () {
    // If we have sites but none selected, select the first one
    if (sitesList.length > 0 && !currentSiteId) {
      selectSite(sitesList[0].id);
    }
  });
}

/**
 * Loads site configurations from IndexedDB or localStorage
 */
function loadSitesFromStorage() {
  // Try to load from localStorage first
  const storedSites = localStorage.getItem(SITES_LIST_KEY);
  if (storedSites) {
    sitesList = JSON.parse(storedSites);
  } else {
    sitesList = [];
  }

  // Load the current site ID
  currentSiteId = localStorage.getItem(CURRENT_SITE_KEY) || null;
}

/**
 * Saves site configurations to storage
 */
function saveSitesToStorage() {
  localStorage.setItem(SITES_LIST_KEY, JSON.stringify(sitesList));
}

/**
 * Saves the current site ID to storage
 */
function saveCurrentSiteToStorage() {
  if (currentSiteId) {
    localStorage.setItem(CURRENT_SITE_KEY, currentSiteId);
  }
}

/**
 * Populates the site selector dropdown with available sites
 */
function populateSiteSelector() {
  const siteSelect = $("#site-select");
  siteSelect.empty();

  // Add default option
  siteSelect.append($("<option>").attr("value", "").text("Select a site"));

  // Add sites from the list
  sitesList.forEach((site) => {
    siteSelect.append($("<option>").attr("value", site.id).text(site.name));
  });

  // Select the current site if available
  if (currentSiteId && sitesList.some((site) => site.id === currentSiteId)) {
    siteSelect.val(currentSiteId);
  }
}

/**
 * Selects a site by ID and updates the current site
 * @param {string} siteId - The ID of the site to select
 */
function selectSite(siteId) {
  currentSiteId = siteId;
  saveCurrentSiteToStorage();

  // Update the site selector
  $("#site-select").val(siteId);

  // You could also load site-specific mappings here if needed
  console.log(`Selected site: ${siteId}`);
}

/**
 * Shows the add site modal
 */
function showAddSiteModal() {
  // Clear the form
  $("#site-name").val("");
  $("#site-url").val("");
  $("#consumer-key").val("");
  $("#consumer-secret").val("");
  $("#edit-site-id").val("");

  // Hide delete button for new sites
  $("#delete-site-btn").addClass("hidden");

  // Set modal title
  $("#modal-title").text("Add New Site");

  // Show the modal
  $("#site-modal").removeClass("hidden");
}

/**
 * Shows the edit site modal for an existing site
 * @param {string} siteId - The ID of the site to edit
 */
function showEditSiteModal(siteId) {
  const site = sitesList.find((s) => s.id === siteId);
  if (!site) return;

  // Fill the form
  $("#site-name").val(site.name);
  $("#site-url").val(site.url);
  $("#consumer-key").val(site.key);
  $("#consumer-secret").val(site.secret);
  $("#edit-site-id").val(site.id);

  // Show delete button for existing sites
  $("#delete-site-btn").removeClass("hidden");

  // Set modal title
  $("#modal-title").text("Edit Site");

  // Show the modal
  $("#site-modal").removeClass("hidden");
}

/**
 * Hides the site modal
 */
function hideModal() {
  $("#site-modal").addClass("hidden");
}

/**
 * Saves a new site or updates an existing one
 */
function saveSite() {
  loadSitesFromStorage();
  const siteName = $("#site-name").val().trim();
  const siteUrl = $("#site-url").val().trim();
  const consumerKey = $("#consumer-key").val().trim();
  const consumerSecret = $("#consumer-secret").val().trim();
  const editSiteId = $("#edit-site-id").val();
  console.log(
    `Saving site: ${siteName}, ${siteUrl}, ${consumerKey}, ${consumerSecret}`
  );

  // Basic validation
  if (!siteName || !siteUrl || !consumerKey || !consumerSecret) {
    alert("Please fill in all fields");
    return;
  }

  // check if same site already exists
  const existingSite = sitesList.find(
    (site) => site.name === siteName && site.id !== editSiteId
  );

  if (editSiteId) {
    // Update existing site
    const siteIndex = sitesList.findIndex((s) => s.id === editSiteId);
    if (siteIndex !== -1) {
      sitesList[siteIndex] = {
        id: editSiteId,
        name: siteName,
        url: siteUrl,
        key: consumerKey,
        secret: consumerSecret,
      };
    }
  } else {
    // Add new site
    const newSite = {
      id: generateUniqueId(),
      name: siteName,
      url: siteUrl,
      key: consumerKey,
      secret: consumerSecret,
    };

    sitesList.push(newSite);

    // If this is the first site, select it automatically
    if (sitesList.length === 1) {
      currentSiteId = newSite.id;
      saveCurrentSiteToStorage();
    }
  }

  // Save sites to storage
  saveSitesToStorage();

  // Update the site selector
  populateSiteSelector();

  // Hide the modal
  hideModal();
}

/**
 * Deletes a site
 */
function deleteSite() {
  const siteId = $("#edit-site-id").val();
  if (!siteId) return;

  // Confirm deletion
  if (!confirm("Are you sure you want to delete this site?")) {
    return;
  }

  // Remove the site from the list
  sitesList = sitesList.filter((site) => site.id !== siteId);

  // If the deleted site was the current site, clear the current site
  if (currentSiteId === siteId) {
    currentSiteId = sitesList.length > 0 ? sitesList[0].id : null;
    saveCurrentSiteToStorage();
  }

  // Save sites to storage
  saveSitesToStorage();

  // Update the site selector
  populateSiteSelector();

  // Hide the modal
  hideModal();
}

/**
 * Generates a unique ID for a new site
 * @returns {string} A unique ID
 */
function generateUniqueId() {
  return "site_" + Date.now() + "_" + Math.random().toString(36).substr(2, 9);
}

/**
 * Gets the currently selected site configuration
 * @returns {Object|null} The current site configuration or null if no site is selected
 */
function getCurrentSite() {
  if (!currentSiteId) return null;
  return sitesList.find((site) => site.id === currentSiteId) || null;
}

/**
 * Checks if a site is selected and shows an error if not
 * @returns {boolean} True if a site is selected, false otherwise
 */
function checkSiteSelected() {
  if (!currentSiteId) {
    alert("Please select a site before proceeding.");
    return false;
  }
  return true;
}

/**
 * Shows the view sites modal with a list of all configured sites
 */
function showViewSitesModal() {
  // Load the latest sites data
  loadSitesFromStorage();

  // Populate the sites table
  populateSitesTable();

  // Show the modal
  $("#view-sites-modal").removeClass("hidden");
}

/**
 * Hides the view sites modal
 */
function hideViewSitesModal() {
  $("#view-sites-modal").addClass("hidden");
}

/**
 * Populates the sites table with all configured sites
 */
function populateSitesTable() {
  const sitesTableBody = $("#sites-list-body");
  const noSitesMessage = $("#no-sites-message");

  // Clear the table
  sitesTableBody.empty();

  // Show message if no sites are configured
  if (sitesList.length === 0) {
    noSitesMessage.removeClass("hidden");
    return;
  }

  // Hide the message if sites exist
  noSitesMessage.addClass("hidden");

  // Add each site to the table
  sitesList.forEach((site) => {
    const row = $("<tr>");

    // Site name column - make it bold if it's the current site
    const nameCell = $("<td>");
    if (site.id === currentSiteId) {
      nameCell.html(
        `<strong>${site.name}</strong> <span class="badge" style="background: #28a745; color: white; padding: 2px 6px; border-radius: 10px; font-size: 12px;">Active</span>`
      );
    } else {
      nameCell.text(site.name);
    }
    row.append(nameCell);

    // URL column
    const urlCell = $("<td>");
    urlCell.text(site.url);
    row.append(urlCell);

    // Actions column
    const actionsCell = $("<td>");

    // Edit button
    const editButton = $("<button>")
      .addClass("edit-site-btn")
      .text("Edit")
      .css({
        background: "#007bff",
        color: "white",
        border: "none",
        padding: "5px 10px",
        "border-radius": "4px",
        cursor: "pointer",
        "margin-right": "5px",
      })
      .on("click", function () {
        hideViewSitesModal();
        showEditSiteModal(site.id);
      });

    // Select button (only show if not the current site)
    const selectButton = $("<button>")
      .addClass("select-site-btn")
      .text("Select")
      .css({
        background: site.id === currentSiteId ? "#6c757d" : "#28a745",
        color: "white",
        border: "none",
        padding: "5px 10px",
        "border-radius": "4px",
        cursor: site.id === currentSiteId ? "not-allowed" : "pointer",
      })
      .prop("disabled", site.id === currentSiteId)
      .on("click", function () {
        selectSite(site.id);
        hideViewSitesModal();
        // Refresh the page to reflect the site change
        // Or you could just update the UI as needed
        populateSiteSelector();
      });

    actionsCell.append(editButton);
    actionsCell.append(selectButton);
    row.append(actionsCell);

    sitesTableBody.append(row);
  });
}

/**
 * Saves the current field mapping to localStorage
 */
function saveFieldMappingToStorage() {
  try {
    localStorage.setItem(FIELD_MAPPING_KEY, JSON.stringify(fieldMapping));
    console.log("Field mapping saved to localStorage");
  } catch (error) {
    console.error("Failed to save field mapping to localStorage:", error);
  }
}

/**
 * Saves the current attribute mapping to localStorage
 */
function saveAttributeMappingToStorage() {
  try {
    localStorage.setItem(
      ATTRIBUTE_MAPPING_KEY,
      JSON.stringify(attributeMappings)
    );
    console.log("Attribute mapping saved to localStorage");
  } catch (error) {
    console.error("Failed to save attribute mapping to localStorage:", error);
  }
}

/**
 * Saves the "new product" checkbox state to localStorage
 */
function saveIsNewUploadToStorage() {
  try {
    localStorage.setItem(IS_NEW_UPLOAD_KEY, $("#is_new_upload").is(":checked"));
    console.log("New product state saved to localStorage");
  } catch (error) {
    console.error("Failed to save new product state to localStorage:", error);
  }
}

/**
 * Retrieves field mapping from localStorage if available
 * @returns {Object|null} The saved field mapping or null if not available
 */
function getSavedFieldMapping() {
  try {
    const savedMapping = localStorage.getItem(FIELD_MAPPING_KEY);
    return savedMapping ? JSON.parse(savedMapping) : null;
  } catch (error) {
    console.error("Failed to retrieve field mapping from localStorage:", error);
    return null;
  }
}

/**
 * Retrieves attribute mapping from localStorage if available
 * @returns {Object|null} The saved attribute mapping or null if not available
 */
function getSavedAttributeMapping() {
  try {
    const savedMapping = localStorage.getItem(ATTRIBUTE_MAPPING_KEY);
    return savedMapping ? JSON.parse(savedMapping) : null;
  } catch (error) {
    console.error(
      "Failed to retrieve attribute mapping from localStorage:",
      error
    );
    return null;
  }
}

/**
 * Retrieves "new product" checkbox state from localStorage if available
 * @returns {boolean} The saved checkbox state or false if not available
 */
function getSavedIsNewUpload() {
  try {
    const isNewUpload = localStorage.getItem(IS_NEW_UPLOAD_KEY);
    return isNewUpload === "true";
  } catch (error) {
    console.error(
      "Failed to retrieve new product state from localStorage:",
      error
    );
    return false;
  }
}

function uploadCSV() {
  const fileInput = $("#csvUpload")[0].files[0];
  if (!fileInput) {
    showUploadStatus("Please select a CSV file", "error");
    return;
  }

  let formData = new FormData();
  formData.append("csv", fileInput);

  showUploadStatus("Uploading...", "");

  $.ajax({
    url: "/upload",
    type: "POST",
    data: formData,
    processData: false,
    contentType: false,
    success: function (response) {
      uploadedFileId = response.data.id;
      showUploadStatus(
        `File '${response.data.filename}' uploaded successfully!`,
        "success"
      );

      // After successful upload, fetch CSV headers
      fetchCSVColumns(uploadedFileId);
    },
    error: function (error) {
      showUploadStatus(
        "Upload failed: " + (error.responseJSON?.message || "Unknown error"),
        "error"
      );
    },
  });
}

function fetchCSVColumns(fileId) {
  $.ajax({
    url: `/columns/${fileId}`,
    type: "GET",
    success: function (response) {
      csvColumns = response.data.headers;

      // Now that we have columns, show the mapping section
      createMappingFields();
      showSection("mapping");
    },
    error: function (error) {
      showUploadStatus(
        "Failed to read CSV headers: " +
          (error.responseJSON?.message || "Unknown error"),
        "error"
      );
    },
  });
}

function createMappingFields() {
  const mappingContainer = $("#mapping-fields");
  mappingContainer.empty();

  // Try to get saved mapping from localStorage
  const savedFieldMapping = getSavedFieldMapping();

  // Create mapping rows for key WooCommerce fields
  woocommerceFields.forEach((field) => {
    const row = $("<tr>");

    // Create field name cell with description tooltip
    const fieldNameCell = $("<td>");
    fieldNameCell.append($("<strong>").text(field.label));

    // Add required indicator if field is required
    if (field.required) {
      fieldNameCell.append(' <span style="color: #dc3545;">*</span>');
    }

    // Add description as helper text
    if (field.description) {
      fieldNameCell.append(
        $("<div>")
          .addClass("field-description")
          .css({
            "font-size": "12px",
            color: "#666",
            "margin-top": "3px",
          })
          .text(field.description)
      );
    }

    const mappingCell = $("<td>");

    const select = $("<select>")
      .attr("id", `mapping-${field.value}`)
      .attr("data-field", field.value)
      .addClass("mapping-select");

    // Only add "Do not import" option for non-required fields
    if (!field.required) {
      // Add empty option
      select.append($("<option>").attr("value", "").text("Do not import"));

      // Add separator
      select.append(
        $("<option>")
          .attr("value", "")
          .text("--------------")
          .prop("disabled", true)
      );
    }

    // Add CSV columns as options
    csvColumns.forEach((column) => {
      const option = $("<option>").attr("value", column).text(column);
      select.append(option);
    });

    // Try to set value from saved mapping first
    let valueSet = false;

    if (savedFieldMapping && savedFieldMapping[field.value]) {
      const savedValue = savedFieldMapping[field.value];
      // Check if the saved column exists in the current CSV
      if (csvColumns.includes(savedValue)) {
        select.val(savedValue);
        valueSet = true;
      }
    }

    // If no value was set from saved mapping, try smart matching
    if (!valueSet) {
      csvColumns.forEach((column) => {
        if (
          column.toLowerCase() === field.value.toLowerCase() ||
          column.toLowerCase().includes(field.value.toLowerCase()) ||
          field.value.toLowerCase().includes(column.toLowerCase())
        ) {
          select.val(column);
          valueSet = true;
        }
      });
    }

    // For required fields, if no match was found, select the first option
    if (field.required && !select.val() && csvColumns.length > 0) {
      select.val(csvColumns[0]);
    }

    select.on("change", function () {
      // For required fields, prevent "Do not import" selection
      if (field.required && !$(this).val()) {
        alert(
          `The field "${field.label}" is required and must be mapped to a CSV column.`
        );
        // Reset to first CSV column if available
        if (csvColumns.length > 0) {
          $(this).val(csvColumns[0]);
        }
      }

      updateFieldMapping();
    });

    mappingCell.append(select);
    row.append(fieldNameCell, mappingCell);
    mappingContainer.append(row);
  });

  // Initialize mapping object
  updateFieldMapping();

  // Clear existing attribute mappings
  $("#attribute-mappings").empty();
  attributeMappings = {};
  attributeCounter = 0;

  // Restore attribute mappings from localStorage if available
  const savedAttributeMapping = getSavedAttributeMapping();
  if (savedAttributeMapping && Object.keys(savedAttributeMapping).length > 0) {
    for (const [name, details] of Object.entries(savedAttributeMapping)) {
      // Only restore attributes whose columns exist in the current CSV
      if (details.column && csvColumns.includes(details.column)) {
        createAttributeRow(name, details.column, details.variable);
      }
    }
  }

  // Restore "new product" checkbox state
  const isNewUpload = getSavedIsNewUpload();
  $("#is_new_upload").prop("checked", isNewUpload);
}

function createAttributeRow(name = "", column = "", isVariable = false) {
  const attributeId = attributeCounter++;
  const container = $("#attribute-mappings");

  const row = $("<div>")
    .addClass("attribute-row")
    .attr("id", `attribute-row-${attributeId}`);

  // Input for attribute name
  const nameInput = $("<input>")
    .attr("type", "text")
    .attr("placeholder", "Attribute Name (e.g. Color)")
    .attr("id", `attribute-name-${attributeId}`)
    .val(name)
    .on("input", function () {
      updateAttributeMappings();
    });

  // Select for CSV column
  const columnSelect = createColumnSelect(
    `attribute-column-${attributeId}`,
    function () {
      updateAttributeMappings();
    }
  );

  if (column && csvColumns.includes(column)) {
    columnSelect.val(column);
  }

  // Variable checkbox
  const checkboxContainer = $("<div>").addClass("attribute-checkbox-container");

  const variableCheckbox = $("<input>")
    .attr("type", "checkbox")
    .attr("id", `attribute-variable-${attributeId}`)
    .prop("checked", isVariable)
    .on("change", function () {
      updateAttributeMappings();
    });

  const checkboxLabel = $("<label>")
    .attr("for", `attribute-variable-${attributeId}`)
    .text("Variable");

  checkboxContainer.append(variableCheckbox, checkboxLabel);

  // Remove button
  const removeBtn = $("<button>")
    .addClass("remove-attribute")
    .text("×")
    .on("click", function () {
      $(`#attribute-row-${attributeId}`).remove();
      delete attributeMappings[attributeId];
      updateAttributeMappings();
    });

  row.append(nameInput, columnSelect, checkboxContainer, removeBtn);
  container.append(row);

  // Update the attribute mappings
  updateAttributeMappings();

  return row;
}

function addAttributeMapping() {
  createAttributeRow();
}

function createColumnSelect(id, onChangeHandler) {
  const select = $("<select>")
    .attr("id", id)
    .addClass("attribute-column-select");

  // Add empty option
  select.append($("<option>").attr("value", "").text("Select CSV column"));

  // Add CSV columns as options
  csvColumns.forEach((column) => {
    select.append($("<option>").attr("value", column).text(column));
  });

  if (onChangeHandler) {
    select.on("change", onChangeHandler);
  }

  return select;
}

function updateAttributeMappings() {
  attributeMappings = {};

  $(".attribute-row").each(function () {
    const id = $(this).attr("id").replace("attribute-row-", "");
    const name = $(`#attribute-name-${id}`).val();
    const column = $(`#attribute-column-${id}`).val();
    const isVariable = $(`#attribute-variable-${id}`).is(":checked");

    if (name && column) {
      attributeMappings[name] = {
        column: column,
        variable: isVariable,
      };
    }
  });

  console.log("Attribute mappings updated:", attributeMappings);

  // Save to localStorage
  saveAttributeMappingToStorage();
}

function updateFieldMapping() {
  fieldMapping = {};

  $(".mapping-select").each(function () {
    const field = $(this).data("field");
    const value = $(this).val();

    if (value) {
      fieldMapping[field] = value;
    }
  });

  console.log("Field mapping updated:", fieldMapping);

  // Save to localStorage
  saveFieldMappingToStorage();
}

function showUploadStatus(message, type) {
  $("#uploadStatus").html(`<div class="${type}">${message}</div>`);
}

function showSection(section) {
  // Hide all sections
  $("#section-upload, #section-mapping, #section-process").addClass("hidden");

  // Show requested section
  $(`#section-${section}`).removeClass("hidden");

  // Update step indicators
  $(".step").removeClass("active completed");

  switch (section) {
    case "upload":
      $("#step1").addClass("active");
      break;
    case "mapping":
      $("#step1").addClass("completed");
      $("#step2").addClass("active");
      break;
    case "process":
      $("#step1, #step2").addClass("completed");
      $("#step3").addClass("active");
      break;
  }
}

function goBack() {
  showSection("upload");
}

function goToMapping() {
  showSection("mapping");
}

function continueToProcessing() {
  // First check if any fields are mapped
  if (Object.keys(fieldMapping).length === 0) {
    alert("Please map at least one field before proceeding.");
    return;
  }

  // Check if all required fields are mapped
  let missingRequiredFields = [];

  woocommerceFields.forEach((field) => {
    if (field.required && !fieldMapping[field.value]) {
      missingRequiredFields.push(field.label);
    }
  });

  if (missingRequiredFields.length > 0) {
    alert(
      `The following required fields must be mapped before proceeding:\n- ${missingRequiredFields.join(
        "\n- "
      )}`
    );
    return;
  }

  // Save "new product" checkbox state
  saveIsNewUploadToStorage();

  showSection("process");
}

function processCSV() {
  if (!uploadedFileId) {
    alert("No CSV file uploaded yet.");
    return;
  }

  if (Object.keys(fieldMapping).length === 0) {
    alert(
      "Field mapping is required. Please go back and map at least one field."
    );
    return;
  }

  let startRow = parseInt($("#startRow").val()) || 0;
  let rowCount = parseInt($("#rowCount").val()) || 99999;
  let priority = parseInt($("#priority").val()) || 1;
  let isNewUpload = $("#is_new_upload").is(":checked");
  const batch_size = parseInt($("#batchSize").val()) || 50;
  const batch_delay_minutes = parseInt($("#batchDelay").val()) || 5;
  const dry_run = $("#dryRun").is(":checked");

  // Create a complete mapping object including attributes
  let completeMapping = { ...fieldMapping };
  console.log("attribute field sent", attributeMappings);

  // Add attributes if any exist
  if (Object.keys(attributeMappings).length > 0) {
    completeMapping.attributes = attributeMappings;
  }

  console.log("wordpress field mapping", completeMapping);
  const siteDetails = getCurrentSite();

  if (!siteDetails) {
    alert("No site selected. Please select a site before processing.");
    return;
  }

  let data = {
    siteDetails,
    startRow,
    rowCount,
    priority,
    wordpress_field_mapping: completeMapping,
    is_new_upload: isNewUpload, // Add the new checkbox value to the data object
    batch_delay_minutes,
    batch_size,
    dry_run,
  };

  $.ajax({
    url: `/process/${uploadedFileId}`,
    type: "POST",
    contentType: "application/json",
    data: JSON.stringify(data),
    success: function (response) {
      $("#processingResults").text(JSON.stringify(response.data, null, 2));

      if (
        response.data.status === "processing" ||
        response.data.status === "queued"
      ) {
        $("#progressText").text("Processing started...");
        resetProgress();
        startTimer();
        trackProgress(uploadedFileId);
      }
    },
    error: function (error) {
      $("#processingResults").text(
        "Failed to start processing: " +
          JSON.stringify(
            error.responseJSON || { message: "Unknown error" },
            null,
            2
          )
      );
      $("#progressText").text("Processing failed to start");
    },
  });
}

function resetProgress() {
  if (currentProgressInterval) {
    clearInterval(currentProgressInterval);
    currentProgressInterval = null;
  }

  // Reset timer
  stopTimer();

  $("#progressBar").css("width", "0%");
  $("#progressText").text("Starting process...");
  $("#processTimer").text("00:00:00");
}

function startTimer() {
  stopTimer(); // Clear any existing timer

  startTime = new Date();
  $("#processTimer").text("00:00:00");

  timerInterval = setInterval(() => {
    const elapsedTime = new Date() - startTime;
    const hours = Math.floor(elapsedTime / 3600000)
      .toString()
      .padStart(2, "0");
    const minutes = Math.floor((elapsedTime % 3600000) / 60000)
      .toString()
      .padStart(2, "0");
    const seconds = Math.floor((elapsedTime % 60000) / 1000)
      .toString()
      .padStart(2, "0");

    $("#processTimer").text(`${hours}:${minutes}:${seconds}`);
  }, 1000);
}

function stopTimer() {
  if (timerInterval) {
    clearInterval(timerInterval);
    timerInterval = null;
  }
}

function trackProgress(id) {
  currentProgressInterval = setInterval(() => {
    $.get(`/progress/${id}`, function (response) {
      let progress = response.data.progress;
      let status = response.data.status;
      let status_message = response.data?.stage_message ?? null;

      $("#progressBar").css("width", progress + "%");
      $("#progressText").text(
        status_message || `Progress: ${progress}% - ${status}`
      );

      if (status === "completed" || progress >= 100) {
        clearInterval(currentProgressInterval);
        currentProgressInterval = null;
        $("#progressText").text("Processing completed");
        stopTimer();

        // Load and display final report
        showFinalReport(id);
      }
    }).fail((error) => {
      clearInterval(currentProgressInterval);
      currentProgressInterval = null;
      $("#progressText").text(
        "Failed to fetch progress: " +
          (error.responseJSON?.message || "Unknown error")
      );
      stopTimer();
    });
  }, 5000);
}

function showFinalReport(fileId) {
  $.ajax({
    url: `/reports/${fileId}`,
    type: "GET",
    success: function (response) {
      const report = response.data.report;
      const finalSummary = `
        <div class="final-report-summary">
          <h4>✅ Processing Complete!</h4>
          <div class="summary-stats">
            <div class="summary-item">
              <span class="summary-label">Total Rows:</span>
              <span class="summary-value">${report.total_rows}</span>
            </div>
            <div class="summary-item">
              <span class="summary-label">Successful:</span>
              <span class="summary-value success">${
                report.successful_rows
              }</span>
            </div>
            <div class="summary-item">
              <span class="summary-label">Failed:</span>
              <span class="summary-value ${
                report.failed_rows > 0 ? "error" : "success"
              }">${report.failed_rows}</span>
            </div>
            <div class="summary-item">
              <span class="summary-label">Success Rate:</span>
              <span class="summary-value">${(
                (report.successful_rows / report.total_rows) *
                100
              ).toFixed(1)}%</span>
            </div>
          </div>
          <div class="report-actions">
            <button onclick="viewReportDetails('${fileId}')" class="btn-view">📊 View Full Report</button>
            <button onclick="switchTab('reports')" class="btn-view">📋 Go to Reports</button>
          </div>
        </div>
      `;

      $("#processingResults").before(finalSummary);
    },
    error: function (error) {
      console.error("Failed to load final report:", error);
    },
  });
}

// Initialize by showing the upload section first
showSection("upload");
// Make sure to call initSites when the page loads
$(document).ready(function () {
  initSites();
});

// Tab Management
function initTabNavigation() {
  $(".nav-tab").on("click", function (e) {
    e.preventDefault();
    const tabId = $(this).data("tab");
    switchTab(tabId);
  });
}

function switchTab(tabId) {
  // Update tab navigation
  $(".nav-tab").removeClass("active");
  $(`.nav-tab[data-tab="${tabId}"]`).addClass("active");

  // Update tab content
  $(".tab-content").addClass("hidden");
  $(`#tab-${tabId}`).removeClass("hidden");

  // Load data when switching to specific tabs
  switch (tabId) {
    case "files":
      loadFilesList();
      break;
    case "reports":
      loadReportsList();
      break;
  }
}

// Files Management
let filesData = [];

function loadFilesList() {
  $.ajax({
    url: "/csv/list",
    type: "GET",
    success: function (response) {
      filesData = response.data.files;
      displayFilesList();
      updateFilesStats();
    },
    error: function (error) {
      console.error("Failed to load files:", error);
      $("#files-list").html(
        '<tr><td colspan="4" class="error">Failed to load files</td></tr>'
      );
    },
  });
}

function displayFilesList() {
  const filesList = $("#files-list");
  const noFilesMessage = $("#no-files-message");

  if (filesData.length === 0) {
    filesList.empty();
    noFilesMessage.removeClass("hidden");
    return;
  }

  noFilesMessage.addClass("hidden");

  const rows = filesData
    .map(
      (file) => `
    <tr>
      <td>
        <strong>${file.filename}</strong>
        <br><small>ID: ${file.id}</small>
      </td>
      <td class="file-size">${formatFileSize(file.size)}</td>
      <td class="date-formatted">${formatDateSafe(file.uploadedAt)}</td>
      <td>
        <div class="table-actions">
          <button class="btn-small btn-view" onclick="useFileForProcessing('${
            file.id
          }', '${file.filename}')">
            📝 Use for Processing
          </button>
          <button class="btn-small btn-delete" onclick="deleteFile('${
            file.filename
          }')">
            🗑️ Delete
          </button>
        </div>
      </td>
    </tr>
  `
    )
    .join("");

  filesList.html(rows);
}

function updateFilesStats() {
  const totalSize = filesData.reduce((sum, file) => sum + file.size, 0);
  $("#files-count").text(
    `${filesData.length} files (${formatFileSize(totalSize)} total)`
  );
}

function refreshFilesList() {
  loadFilesList();
}

function deleteFile(filename) {
  if (!confirm(`Are you sure you want to delete "${filename}"?`)) {
    return;
  }

  $.ajax({
    url: `/csv/${filename}`,
    type: "DELETE",
    success: function (response) {
      loadFilesList(); // Refresh the list
      alert("File deleted successfully");
    },
    error: function (error) {
      alert(
        "Failed to delete file: " +
          (error.responseJSON?.message || "Unknown error")
      );
    },
  });
}

function useFileForProcessing(fileId, filename) {
  // Switch to processor tab
  switchTab("processor");

  // Set the uploaded file ID and fetch columns
  uploadedFileId = fileId;
  fetchCSVColumns(fileId);

  // Update upload status
  showUploadStatus(`Using existing file: ${filename}`, "success");

  // Show a confirmation
  alert(
    `Now using "${filename}" for processing. You can proceed to field mapping.`
  );
}

// Reports Management
let reportsData = [];

function loadReportsList() {
  $.ajax({
    url: "/reports/list",
    type: "GET",
    success: function (response) {
      reportsData = response.data.reports;
      displayReportsList();
      updateReportsStats();
    },
    error: function (error) {
      console.error("Failed to load reports:", error);
      $("#reports-list").html(
        '<tr><td colspan="6" class="error">Failed to load reports</td></tr>'
      );
    },
  });
}

function displayReportsList() {
  const reportsList = $("#reports-list");
  const noReportsMessage = $("#no-reports-message");

  if (reportsData.length === 0) {
    reportsList.empty();
    noReportsMessage.removeClass("hidden");
    return;
  }

  noReportsMessage.addClass("hidden");

  const rows = reportsData
    .map((report) => {
      // Try to get report content to extract additional info
      return `
      <tr>
        <td>
          <strong>${report.fileId}</strong>
          <br><small>${report.filename}</small>
        </td>
        <td>
          <span class="status-badge status-completed">Completed</span>
        </td>
        <td>
          <div class="table-progress">
            <div class="table-progress-bar" style="width: 100%"></div>
          </div>
          <small>100%</small>
        </td>
        <td>
          <small>View details for stats</small>
        </td>
        <td class="date-formatted">${formatDateSafe(report.createdAt)}</td>
        <td>
          <div class="table-actions">
            <button class="btn-small btn-view" onclick="viewReportDetails('${
              report.fileId
            }')">
              📊 View Details
            </button>
            <button class="btn-small btn-delete" onclick="deleteReport('${
              report.fileId
            }')">
              🗑️ Delete
            </button>
          </div>
        </td>
      </tr>
    `;
    })
    .join("");

  reportsList.html(rows);
}

function updateReportsStats() {
  $("#reports-count").text(`${reportsData.length} reports available`);
}

function refreshReportsList() {
  loadReportsList();
}

function viewReportDetails(fileId) {
  $.ajax({
    url: `/reports/${fileId}`,
    type: "GET",
    success: function (response) {
      displayReportDetailsModal(response.data.report);
    },
    error: function (error) {
      alert(
        "Failed to load report details: " +
          (error.responseJSON?.message || "Unknown error")
      );
    },
  });
}

// In your main JavaScript file - Update the displayReportDetailsModal function

function displayReportDetailsModal(reportData) {
  const content = $("#report-details-content");

  // Use safe formatting functions
  const startTimeFormatted = formatDateTimeSafe(reportData.start_time);
  const endTimeFormatted = formatDateTimeSafe(reportData.last_updated);
  const duration = formatDurationSafe(
    reportData.start_time,
    reportData.last_updated
  );

  // NEW: Generate failed items details
  const failedRowsHtml =
    reportData.failed_row_details && reportData.failed_row_details.length > 0
      ? `
      <div class="failed-items-section">
        <h5>❌ Failed Rows (${reportData.failed_row_details.length})</h5>
        <div class="failed-items-list">
          ${reportData.failed_row_details
            .slice(0, 10)
            .map(
              (item) =>
                `<div class="failed-item">
              <strong>Row ${item.row_number}:</strong> ${item.reason}
            </div>`
            )
            .join("")}
          ${
            reportData.failed_row_details.length > 10
              ? `<div class="failed-item-more">... and ${
                  reportData.failed_row_details.length - 10
                } more</div>`
              : ""
          }
        </div>
      </div>
    `
      : "";

  const failedProductsHtml =
    reportData.failed_product_details &&
    reportData.failed_product_details.length > 0
      ? `
      <div class="failed-items-section">
        <h5>🛍️ Failed Products (${
          reportData.failed_product_details.length
        })</h5>
        <div class="failed-items-list">
          ${reportData.failed_product_details
            .slice(0, 10)
            .map(
              (item) =>
                `<div class="failed-item">
              <strong>SKU ${item.sku}:</strong> ${item.reason}
            </div>`
            )
            .join("")}
          ${
            reportData.failed_product_details.length > 10
              ? `<div class="failed-item-more">... and ${
                  reportData.failed_product_details.length - 10
                } more</div>`
              : ""
          }
        </div>
      </div>
    `
      : "";

  const html = `
    <div class="report-section">
      <h4>📈 Processing Summary</h4>
      <div class="report-stats">
        <div class="stat-item">
          <div class="stat-value">${reportData.total_rows}</div>
          <div class="stat-label">Total Rows</div>
        </div>
        <div class="stat-item">
          <div class="stat-value">${reportData.successful_rows}</div>
          <div class="stat-label">Successful</div>
        </div>
        <div class="stat-item">
          <div class="stat-value">${reportData.failed_rows}</div>
          <div class="stat-label">Failed</div>
        </div>
        <div class="stat-item">
          <div class="stat-value">${reportData.percent.toFixed(1)}%</div>
          <div class="stat-label">Progress</div>
        </div>
      </div>
    </div>
    
    ${failedRowsHtml}
    ${failedProductsHtml}
    
    <div class="report-section">
      <h4>⏱️ Timing Information</h4>
      <p><strong>Started:</strong> ${startTimeFormatted}</p>
      <p><strong>Completed:</strong> ${endTimeFormatted}</p>
      <p><strong>Duration:</strong> ${duration}</p>
    </div>
    
    <div class="report-section">
      <h4>📊 Current Stage</h4>
      <p><strong>Status:</strong> ${
        typeof reportData.stage === "string"
          ? reportData.stage
          : JSON.stringify(reportData.stage)
      }</p>
    </div>
    
    <div class="report-section">
      <h4>🔍 Technical Details</h4>
      <pre style="background: #f8f9fa; padding: 10px; border-radius: 4px; overflow-x: auto;">${JSON.stringify(
        reportData,
        null,
        2
      )}</pre>
    </div>
  `;

  content.html(html);
  $("#report-details-modal").removeClass("hidden");
}

function hideReportModal() {
  $("#report-details-modal").addClass("hidden");
}

function deleteReport(fileId) {
  if (!confirm(`Are you sure you want to delete the report for "${fileId}"?`)) {
    return;
  }

  $.ajax({
    url: `/reports/${fileId}`,
    type: "DELETE",
    success: function (response) {
      loadReportsList(); // Refresh the list
      alert("Report deleted successfully");
    },
    error: function (error) {
      alert(
        "Failed to delete report: " +
          (error.responseJSON?.message || "Unknown error")
      );
    },
  });
}

// Utility Functions
function formatFileSize(bytes) {
  if (bytes === 0) return "0 Bytes";

  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

function formatDate(dateString) {
  const date = new Date(dateString);
  return (
    date.toLocaleDateString() +
    " " +
    date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
  );
}

function formatDateTime(date) {
  return date.toLocaleDateString() + " at " + date.toLocaleTimeString();
}

function formatDuration(milliseconds) {
  const seconds = Math.floor(milliseconds / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m ${seconds % 60}s`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
}

// Update the existing $(document).ready function
$(document).ready(function () {
  initSites();
  initTabNavigation();

  // Load files and reports data on page load
  loadFilesList();
  loadReportsList();
});

// Utility function to convert Rust timestamp to JavaScript Date
function rustTimestampToDate(rustTimestamp) {
  if (!rustTimestamp || typeof rustTimestamp !== "object") {
    return new Date(); // Return current date as fallback
  }

  // Convert seconds to milliseconds and add nanoseconds converted to milliseconds
  const milliseconds =
    rustTimestamp.secs_since_epoch * 1000 +
    rustTimestamp.nanos_since_epoch / 1000000;
  return new Date(milliseconds);
}

// Enhanced format functions that handle both string and Rust timestamp formats
function formatDateSafe(dateInput) {
  let date;

  if (typeof dateInput === "string") {
    date = new Date(dateInput);
  } else if (typeof dateInput === "object" && dateInput.secs_since_epoch) {
    date = rustTimestampToDate(dateInput);
  } else {
    return "Invalid Date";
  }

  if (isNaN(date.getTime())) {
    return "Invalid Date";
  }

  return (
    date.toLocaleDateString() +
    " " +
    date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
  );
}

function formatDateTimeSafe(dateInput) {
  let date;

  if (typeof dateInput === "string") {
    date = new Date(dateInput);
  } else if (typeof dateInput === "object" && dateInput.secs_since_epoch) {
    date = rustTimestampToDate(dateInput);
  } else {
    return "Invalid Date";
  }

  if (isNaN(date.getTime())) {
    return "Invalid Date";
  }

  return date.toLocaleDateString() + " at " + date.toLocaleTimeString();
}

function formatDurationSafe(startTime, endTime) {
  let startDate, endDate;

  // Handle start time
  if (typeof startTime === "string") {
    startDate = new Date(startTime);
  } else if (typeof startTime === "object" && startTime.secs_since_epoch) {
    startDate = rustTimestampToDate(startTime);
  } else {
    return "Invalid Duration";
  }

  // Handle end time
  if (typeof endTime === "string") {
    endDate = new Date(endTime);
  } else if (typeof endTime === "object" && endTime.secs_since_epoch) {
    endDate = rustTimestampToDate(endTime);
  } else {
    return "Invalid Duration";
  }

  if (isNaN(startDate.getTime()) || isNaN(endDate.getTime())) {
    return "Invalid Duration";
  }

  const milliseconds = endDate - startDate;
  return formatDuration(milliseconds);
}
