// State management
let currentConfig = null;
let currentNotecardId = 1;
let hasUnsavedChanges = false;

// DOM elements
const elements = {
  loadingOverlay: document.getElementById('loadingOverlay'),
  connectionStatus: document.getElementById('connectionStatus'),
  connectionText: document.getElementById('connectionText'),
  saveBtn: document.getElementById('saveBtn'),
  clearBtn: document.getElementById('clearBtn'),
  notecardButtons: document.getElementById('notecardButtons'),
  currentNotecardId: document.getElementById('currentNotecardId'),
  notecardContent: document.getElementById('notecardContent'),
  charCount: document.getElementById('charCount'),
  launchOnStartup: document.getElementById('launchOnStartup'),
  hotkeyModifiers: document.getElementById('hotkeyModifiers'),
  hotkeyDisplay: document.getElementById('hotkeyDisplay'),
  opacity: document.getElementById('opacity'),
  opacityValue: document.getElementById('opacityValue'),
  opacityPreview: document.getElementById('opacityPreview'),
  fontSize: document.getElementById('fontSize'),
  fontSizeValue: document.getElementById('fontSizeValue'),
  autoHide: document.getElementById('autoHide'),
  autoHideValue: document.getElementById('autoHideValue'),
  fontFamily: document.getElementById('fontFamily'),
  algorithmicSpacing: document.getElementById('algorithmicSpacing'),
  aboutModal: document.getElementById('aboutModal'),
  toastContainer: document.getElementById('toastContainer')
};

// Initialize Bootstrap components
const aboutModalInstance = new bootstrap.Modal(elements.aboutModal);
const tooltipTriggerList = [].slice.call(document.querySelectorAll('[data-bs-toggle="tooltip"]'));
const tooltipList = tooltipTriggerList.map(el => new bootstrap.Tooltip(el));

// Platform-specific adjustments
if (window.notecognitoAPI.platform === 'darwin') {
  document.body.classList.add('platform-darwin');
}

// Initialize the application
async function initialize() {
  try {
    // Connect to core service
    const connectResult = await window.notecognitoAPI.connectToCore();
    if (!connectResult.success) {
      throw new Error(connectResult.error || 'Failed to connect to core service');
    }

    // Get current configuration
    const configResult = await window.notecognitoAPI.getConfiguration();
    if (!configResult.success) {
      throw new Error(configResult.error || 'Failed to load configuration');
    }

    currentConfig = configResult.config;
    updateConnectionStatus(true);
    setupUI();
    loadConfiguration();
    hideLoadingOverlay();

  } catch (error) {
    console.error('Initialization error:', error);
    showToast('Failed to connect to Notecognito service. Please ensure the service is running.', 'danger');
    updateConnectionStatus(false);
    hideLoadingOverlay();
  }
}

// Update connection status indicator
function updateConnectionStatus(connected) {
  if (connected) {
    elements.connectionStatus.classList.remove('disconnected');
    elements.connectionStatus.classList.add('connected');
    elements.connectionText.textContent = 'Connected';
    elements.saveBtn.disabled = false;
  } else {
    elements.connectionStatus.classList.remove('connected');
    elements.connectionStatus.classList.add('disconnected');
    elements.connectionText.textContent = 'Disconnected';
    elements.saveBtn.disabled = true;
  }
}

// Hide loading overlay
function hideLoadingOverlay() {
  elements.loadingOverlay.style.display = 'none';
}

// Setup UI elements
function setupUI() {
  // Create notecard buttons
  for (let i = 1; i <= 9; i++) {
    const button = document.createElement('button');
    button.className = 'btn btn-outline-primary notecard-button';
    button.textContent = i;
    button.dataset.notecardId = i;

    button.addEventListener('click', () => selectNotecard(i));

    elements.notecardButtons.appendChild(button);
  }

  // Select first notecard
  selectNotecard(1);

  // Setup event listeners
  elements.notecardContent.addEventListener('input', handleContentChange);
  elements.saveBtn.addEventListener('click', saveConfiguration);
  elements.clearBtn.addEventListener('click', clearCurrentNotecard);
  elements.launchOnStartup.addEventListener('change', markAsChanged);
  elements.hotkeyModifiers.addEventListener('change', handleHotkeyChange);

  // Display settings listeners
  elements.opacity.addEventListener('input', handleOpacityChange);
  elements.fontSize.addEventListener('input', handleFontSizeChange);
  elements.autoHide.addEventListener('input', handleAutoHideChange);
  elements.fontFamily.addEventListener('change', markAsChanged);
  elements.algorithmicSpacing.addEventListener('change', markAsChanged);

  // Listen for menu actions
  window.notecognitoAPI.onMenuAction((action) => {
    switch (action) {
      case 'save':
        saveConfiguration();
        break;
      case 'about':
        aboutModalInstance.show();
        break;
    }
  });

  // Warn before closing with unsaved changes
  window.addEventListener('beforeunload', (e) => {
    if (hasUnsavedChanges) {
      e.returnValue = 'You have unsaved changes. Are you sure you want to leave?';
    }
  });
}

// Load configuration into UI
function loadConfiguration() {
  if (!currentConfig) return;

  // Global settings
  elements.launchOnStartup.checked = currentConfig.launch_on_startup;

  // Set hotkey modifiers
  const modifiers = currentConfig.hotkey_modifiers || ['Control', 'Shift'];
  Array.from(elements.hotkeyModifiers.options).forEach(option => {
    option.selected = modifiers.includes(option.value);
  });
  updateHotkeyDisplay();

  // Default display properties
  const defaults = currentConfig.default_display_properties;
  elements.opacity.value = defaults.opacity;
  elements.fontSize.value = defaults.font_size;
  elements.autoHide.value = defaults.auto_hide_duration;
  elements.fontFamily.value = defaults.font_family;
  elements.algorithmicSpacing.checked = defaults.algorithmic_spacing;

  // Update display values
  handleOpacityChange();
  handleFontSizeChange();
  handleAutoHideChange();

  // Load current notecard
  loadNotecard(currentNotecardId);
}

// Select a notecard
function selectNotecard(id) {
  // Save current notecard if changed
  if (hasUnsavedChanges) {
    saveCurrentNotecard();
  }

  currentNotecardId = id;

  // Update button states
  document.querySelectorAll('.notecard-button').forEach(btn => {
    if (parseInt(btn.dataset.notecardId) === id) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });

  elements.currentNotecardId.textContent = id;
  loadNotecard(id);
}

// Load a notecard
function loadNotecard(id) {
  const notecard = currentConfig.notecards[id.toString()];
  if (notecard) {
    elements.notecardContent.value = notecard.content || '';
    updateCharCount();
  }
}

// Handle content change
function handleContentChange() {
  updateCharCount();
  markAsChanged();
}

// Update character count
function updateCharCount() {
  const length = elements.notecardContent.value.length;
  elements.charCount.textContent = length;

  if (length > 10000) {
    elements.charCount.classList.add('text-danger');
  } else {
    elements.charCount.classList.remove('text-danger');
  }
}

// Handle hotkey change
function handleHotkeyChange() {
  updateHotkeyDisplay();
  markAsChanged();
}

// Update hotkey display
function updateHotkeyDisplay() {
  const selected = Array.from(elements.hotkeyModifiers.selectedOptions)
    .map(option => window.notecognitoAPI.getHotkeyDisplayName(option.value))
    .filter(Boolean)
    .join('+');

  elements.hotkeyDisplay.textContent = selected + '+[1-9]';
}

// Handle opacity change
function handleOpacityChange() {
  const value = elements.opacity.value;
  elements.opacityValue.textContent = value;
  elements.opacityPreview.style.opacity = value / 100;
  markAsChanged();
}

// Handle font size change
function handleFontSizeChange() {
  const value = elements.fontSize.value;
  elements.fontSizeValue.textContent = value;
  markAsChanged();
}

// Handle auto-hide change
function handleAutoHideChange() {
  const value = elements.autoHide.value;
  if (value === '0') {
    elements.autoHideValue.textContent = 'Disabled';
  } else {
    elements.autoHideValue.textContent = `${value}s`;
  }
  markAsChanged();
}

// Mark as having unsaved changes
function markAsChanged() {
  hasUnsavedChanges = true;
  elements.saveBtn.classList.add('btn-warning');
  elements.saveBtn.classList.remove('btn-primary');
}

// Save current notecard to config
function saveCurrentNotecard() {
  const notecard = {
    id: currentNotecardId,
    content: elements.notecardContent.value
  };

  currentConfig.notecards[currentNotecardId.toString()] = notecard;
}

// Clear current notecard
function clearCurrentNotecard() {
  if (confirm('Are you sure you want to clear this notecard?')) {
    elements.notecardContent.value = '';
    updateCharCount();
    markAsChanged();
  }
}

// Save configuration
async function saveConfiguration() {
  try {
    // Save current notecard
    saveCurrentNotecard();

    // Update config from UI
    currentConfig.launch_on_startup = elements.launchOnStartup.checked;
    currentConfig.hotkey_modifiers = Array.from(elements.hotkeyModifiers.selectedOptions)
      .map(option => option.value);

    currentConfig.default_display_properties = {
      opacity: parseInt(elements.opacity.value),
      position: currentConfig.default_display_properties.position,
      size: currentConfig.default_display_properties.size,
      auto_hide_duration: parseInt(elements.autoHide.value),
      font_family: elements.fontFamily.value,
      font_size: parseInt(elements.fontSize.value),
      algorithmic_spacing: elements.algorithmicSpacing.checked
    };

    // Save to core
    const result = await window.notecognitoAPI.saveConfiguration(currentConfig);

    if (result.success) {
      hasUnsavedChanges = false;
      elements.saveBtn.classList.remove('btn-warning');
      elements.saveBtn.classList.add('btn-primary');
      showToast('Configuration saved successfully!', 'success');
    } else {
      throw new Error(result.error || 'Failed to save configuration');
    }
  } catch (error) {
    console.error('Save error:', error);
    showToast('Failed to save configuration: ' + error.message, 'danger');
  }
}

// Show toast notification
function showToast(message, type = 'info') {
  const toastId = `toast-${Date.now()}`;
  const toastHtml = `
    <div id="${toastId}" class="toast" role="alert">
      <div class="toast-header">
        <i class="bi bi-${type === 'success' ? 'check-circle' : 'exclamation-circle'} text-${type} me-2"></i>
        <strong class="me-auto">Notecognito</strong>
        <button type="button" class="btn-close" data-bs-dismiss="toast"></button>
      </div>
      <div class="toast-body">
        ${message}
      </div>
    </div>
  `;

  elements.toastContainer.insertAdjacentHTML('beforeend', toastHtml);

  const toastElement = document.getElementById(toastId);
  const toast = new bootstrap.Toast(toastElement, { autohide: true, delay: 3000 });
  toast.show();

  toastElement.addEventListener('hidden.bs.toast', () => {
    toastElement.remove();
  });
}

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', initialize);