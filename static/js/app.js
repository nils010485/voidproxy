// ============================================================================
// VOIDPROXY MANAGER - CLEAN ARCHITECTURE
// ============================================================================

class VoidProxyManager {
    constructor() {
        this.instances = new Map();
        this.editingId = null;
        this.init();
    }

    init() {
        console.log('VoidProxy Manager initialized');
        this.setupEventListeners();
        this.loadInstances();
        this.injectIcons();
        this.startStatsRefresh();
    }

    setupEventListeners() {
        // IP filter type change
        const ipFilterType = document.getElementById('ipFilterType');
        if (ipFilterType) {
            ipFilterType.addEventListener('change', () => {
                const ipList = document.getElementById('ipList');
                if (ipList) {
                    ipList.style.display = ipFilterType.value === 'none' ? 'none' : 'block';
                }
            });
        }

        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.hideModal();
            }
        });
    }

    injectIcons() {
        // Inject all icons using the new IconSystem
        const iconMap = {
            'logo-icon': 'network',
            'new-instance-icon': 'plus',
            'server-icon': 'server',
            'inactive-icon': 'pause-circle',
            'total-icon': 'database',
            'traffic-icon': 'exchange',
            'empty-network-icon': 'network',
            'empty-plus-icon': 'plus',
            'form-tag-icon': 'tag',
            'form-location-icon': 'map-marker-alt',
            'form-door-open-icon': 'door-open',
            'form-globe-icon': 'globe',
            'form-door-closed-icon': 'door-closed',
            'form-exchange-icon': 'exchange',
            'form-shield-icon': 'shield',
            'form-clock-icon': 'clock',
            'form-hourglass-icon': 'hourglass',
            'form-log-icon': 'log',
            'modal-cancel-icon': 'times',
            'modal-save-icon': 'save',
            'modal-close-icon': 'times'
        };

        Object.entries(iconMap).forEach(([elementId, iconName]) => {
            const element = document.getElementById(elementId);
            if (element) {
                const className = elementId.includes('empty-') ? 'icon-xl' :
                                 elementId.includes('form-') ? '' : 'icon-lg';
                element.innerHTML = IconSystem.create(iconName, className);
            }
        });
    }

    // Modal Management
    showModal(title = 'Create New Proxy Instance', instance = null) {
        this.editingId = instance ? instance.id : null;

        // Update modal title and button text
        const modalTitle = document.getElementById('modalTitle');
        const saveButtonText = document.getElementById('saveButtonText');

        if (modalTitle) modalTitle.textContent = title;
        if (saveButtonText) saveButtonText.textContent = instance ? 'Update Instance' : 'Create Instance';

        // Fill form if editing
        if (instance) {
            this.fillForm(instance);
        } else {
            this.resetForm();
        }

        // Show modal
        const modal = document.getElementById('modalOverlay');
        if (modal) {
            modal.classList.add('active');
            document.body.style.overflow = 'hidden';
        }
    }

    hideModal() {
        const modal = document.getElementById('modalOverlay');
        if (modal) {
            modal.classList.remove('active');
            document.body.style.overflow = 'auto';
            this.editingId = null;
            this.resetForm();
        }
    }

    fillForm(instance) {
        const fields = {
            'instanceName': instance.name,
            'listenIp': instance.config.proxy.listen_ip,
            'listenPort': instance.config.proxy.listen_port,
            'dstIp': instance.config.proxy.dst_ip,
            'dstPort': instance.config.proxy.dst_port,
            'instanceProtocol': instance.config.proxy.protocol.toLowerCase(),
            'autoStart': instance.auto_start,
            'connectTimeout': instance.config.proxy.connect_timeout_secs,
            'idleTimeout': instance.config.proxy.idle_timeout_secs,
            'logLevel': instance.config.proxy.log_level.toLowerCase()
        };

        Object.entries(fields).forEach(([field, value]) => {
            const element = document.getElementById(field);
            if (element) {
                if (element.type === 'checkbox') {
                    element.checked = value;
                } else {
                    element.value = value;
                }
            }
        });

        // Handle IP filtering
        const ipFilterType = document.getElementById('ipFilterType');
        const ipList = document.getElementById('ipList');

        // Check if allow_list or deny_list exists in config
        if (instance.config.ip_filter && instance.config.ip_filter.allow_list && instance.config.ip_filter.allow_list.length > 0) {
            if (ipFilterType) ipFilterType.value = 'allow';
            if (ipList) {
                ipList.value = instance.config.ip_filter.allow_list.join('\n');
                ipList.style.display = 'block';
            }
        } else if (instance.config.ip_filter && instance.config.ip_filter.deny_list && instance.config.ip_filter.deny_list.length > 0) {
            if (ipFilterType) ipFilterType.value = 'deny';
            if (ipList) {
                ipList.value = instance.config.ip_filter.deny_list.join('\n');
                ipList.style.display = 'block';
            }
        } else {
            if (ipFilterType) ipFilterType.value = 'none';
            if (ipList) ipList.style.display = 'none';
        }
    }

    resetForm() {
        const form = document.getElementById('instanceForm');
        if (form) form.reset();

        const ipList = document.getElementById('ipList');
        if (ipList) {
            ipList.style.display = 'none';
            ipList.value = '';
        }
    }

    // Instance Management
    async loadInstances() {
        try {
            const response = await fetch(`${window.API_BASE_URL}/api/instances`);
            if (!response.ok) throw new Error('Failed to load instances');

            const instances = await response.json();

            // Preserve existing metrics when updating instances
            instances.forEach(instance => {
                const existing = this.instances.get(instance.id);
                if (existing) {
                    // Keep the existing metrics data
                    instance.bytes_sent = existing.bytes_sent || 0;
                    instance.bytes_received = existing.bytes_received || 0;
                    instance.bytes_sent_per_sec = existing.bytes_sent_per_sec || 0;
                    instance.bytes_received_per_sec = existing.bytes_received_per_sec || 0;
                      instance.session_metrics = existing.session_metrics;
                }
            });

            this.instances = new Map(instances.map(instance => [instance.id, instance]));
            this.renderInstances();
            this.updateStats();
        } catch (error) {
            console.error('Error loading instances:', error);
            ToastSystem.show('Failed to load instances', 'error');
        }
    }

    renderInstances() {
        const tbody = document.getElementById('instancesTableBody');
        if (!tbody) return;

        if (this.instances.size === 0) {
            tbody.innerHTML = `
                <tr>
                    <td colspan="7" style="text-align: center; padding: var(--spacing-16);">
                        <div class="empty-state">
                            <div class="empty-icon">
                                ${IconSystem.create('network', 'icon-xl')}
                            </div>
                            <div class="empty-title">No Proxy Instances</div>
                            <div class="empty-description">Create your first proxy instance to get started</div>
                            <button class="btn btn-primary" onclick="proxyManager.showModal()">
                                <span class="btn-icon">${IconSystem.create('plus')}</span>
                                Create Instance
                            </button>
                        </div>
                    </td>
                </tr>
            `;
            return;
        }

        tbody.innerHTML = Array.from(this.instances.values()).map(instance => `
            <tr>
                <td>
                    <strong>${Utils.escapeHtml(instance.name)}</strong>
                </td>
                <td>${Utils.escapeHtml(instance.config.proxy.listen_ip)}:${instance.config.proxy.listen_port}</td>
                <td>${Utils.escapeHtml(instance.config.proxy.dst_ip)}:${instance.config.proxy.dst_port}</td>
                <td>
                    <span class="status-badge ${instance.status === 'running' ? 'active' : 'inactive'}">
                        <span class="status-dot"></span>
                        ${(instance.config.proxy.protocol || 'tcp').toUpperCase()}
                    </span>
                </td>
                <td>
                    <span class="status-badge ${instance.status === 'running' ? 'active' : 'inactive'}">
                        <span class="status-dot"></span>
                        ${instance.status.charAt(0).toUpperCase() + instance.status.slice(1)}
                    </span>
                </td>
                <td>
                    <div class="traffic-info">
                        <div class="traffic-total">${Utils.formatBytes((instance.bytes_sent || 0) + (instance.bytes_received || 0))}</div>
                        <div class="traffic-rate" data-instance="${instance.id}">
                            ${(instance.bytes_sent_per_sec || instance.bytes_received_per_sec) ?
                                `${Utils.formatBytes((instance.bytes_sent_per_sec || 0) + (instance.bytes_received_per_sec || 0))}/s` :
                                '-'}
                        </div>
                    </div>
                      <div class="session-metrics"></div>
                </td>
                <td>
                    <div class="table-actions">
                        ${instance.status === 'running' ?
                            `<button class="btn btn-sm" onclick="proxyManager.toggleInstance('${instance.id}')" title="Stop">
                                ${IconSystem.create('stop')}
                            </button>` :
                            `<button class="btn btn-sm" onclick="proxyManager.toggleInstance('${instance.id}')" title="Start">
                                ${IconSystem.create('play')}
                            </button>`
                        }
                        <button class="btn btn-sm" onclick="proxyManager.editInstance('${instance.id}')" title="Edit">
                            ${IconSystem.create('edit')}
                        </button>
                        <button class="btn btn-sm btn-danger" onclick="proxyManager.deleteInstance('${instance.id}')" title="Delete">
                            ${IconSystem.create('trash')}
                        </button>
                    </div>
                </td>
            </tr>
        `).join('');
    }

    updateStats() {
        const active = Array.from(this.instances.values()).filter(i => i.status === 'running').length;
        const inactive = this.instances.size - active;
        const totalTraffic = Array.from(this.instances.values())
            .reduce((sum, i) => sum + (i.bytes_sent || 0) + (i.bytes_received || 0), 0);

        const updateElement = (id, value) => {
            const element = document.getElementById(id);
            if (element) element.textContent = value;
        };

        updateElement('activeCount', active);
        updateElement('inactiveCount', inactive);
        updateElement('totalCount', this.instances.size);
        updateElement('trafficCount', Utils.formatBytes(totalTraffic));
    }

    async loadStats() {
        try {
            const response = await fetch(`${window.API_BASE_URL}/api/stats`);
            if (!response.ok) return; // Silently fail if stats endpoint not available

            const stats = await response.json();
            this.updateInstancesWithStats(stats);
            this.updateStats();

            // Load detailed metrics for running instances
            await this.loadDetailedMetrics();
        } catch (error) {
            console.debug('Stats refresh failed (endpoint may not be ready yet):', error);
        }
    }

    async loadDetailedMetrics() {
        try {
            // Get running instances
            const runningInstances = Array.from(this.instances.values()).filter(i => i.status === 'running');

            // Load session metrics for each running instance
            for (const instance of runningInstances) {
                try {
                    const sessionMetrics = await fetch(`${window.API_BASE_URL}/api/instances/${instance.id}/session-metrics`);

                    if (sessionMetrics.ok) {
                        const sessionData = await sessionMetrics.json();
                        this.updateInstanceSessionMetrics(instance.id, sessionData);
                    }
                } catch (error) {
                    console.debug(`Failed to load session metrics for instance ${instance.id}:`, error);
                }
            }
        } catch (error) {
            console.debug('Failed to load detailed metrics:', error);
        }
    }

  
    updateInstanceSessionMetrics(instanceId, metrics) {
        const instance = this.instances.get(instanceId);
        if (!instance) return;

        // Update instance with session metrics
        instance.session_metrics = metrics;

        // Update session info in the UI (for UDP instances)
        const row = document.querySelector(`tr:has(button[onclick*="${instanceId}"])`);
        if (row && instance.config && instance.config.proxy && instance.config.proxy.protocol.includes('udp')) {
            const sessionCell = row.querySelector('.session-metrics');
            if (sessionCell) {
                sessionCell.innerHTML = `
                    <div class="session-info">
                        <span class="session-count">${metrics.active_sessions} sessions</span>
                        <span class="session-timeout">Timeout: ${metrics.session_timeout_seconds}s</span>
                    </div>
                `;
            }
        }
    }

    updateInstancesWithStats(stats) {
        // Update local instances with fresh stats without full reload
        Object.entries(stats).forEach(([instanceId, statData]) => {
            const instance = this.instances.get(instanceId);
            if (instance) {
                // Update metrics
                instance.bytes_sent = statData.bytes_sent;
                instance.bytes_received = statData.bytes_received;
                instance.status = statData.status;

                // Update status badge in DOM if changed
                const statusCell = document.querySelector(`tr:has(button[onclick*="${instanceId}"]) td:nth-child(5) .status-badge`);
                if (statusCell) {
                    statusCell.className = `status-badge ${statData.status === 'running' ? 'active' : 'inactive'}`;
                    const statusText = statusCell.querySelector('.status-dot').nextSibling;
                    if (statusText) {
                        statusText.textContent = statData.status.charAt(0).toUpperCase() + statData.status.slice(1);
                    }
                }

                // Update traffic cell
                const trafficCell = document.querySelector(`tr:has(button[onclick*="${instanceId}"]) td:nth-child(6)`);
                if (trafficCell) {
                    const totalElement = trafficCell.querySelector('.traffic-total');
                    const rateElement = trafficCell.querySelector('.traffic-rate');

                    if (totalElement) {
                        totalElement.textContent = Utils.formatBytes((statData.bytes_sent || 0) + (statData.bytes_received || 0));
                    }
                    if (rateElement) {
                        const totalRate = (statData.bytes_sent_per_sec || 0) + (statData.bytes_received_per_sec || 0);
                        rateElement.textContent = totalRate > 0 ? `${Utils.formatBytes(totalRate)}/s` : '-';

                        // Add visual indicators for traffic levels
                        rateElement.classList.remove('high-traffic', 'medium-traffic');
                        if (totalRate > 1024 * 1024) { // > 1MB/s
                            rateElement.classList.add('high-traffic');
                        } else if (totalRate > 1024 * 100) { // > 100KB/s
                            rateElement.classList.add('medium-traffic');
                        }
                    }
                }
            }
        });
    }

    startStatsRefresh() {
        // Load stats metrics every 2 seconds for better performance
        setInterval(() => {
            this.loadStats();
        }, 2000);

        // Load full instances data every 30 seconds (less frequent to avoid overwriting stats)
        setInterval(() => {
            this.loadInstances();
        }, 30000);
    }

    // Actions
    async toggleInstance(id) {
        const instance = this.instances.get(id);
        if (!instance) return;

        try {
            const action = instance.status === 'running' ? 'stop' : 'start';
            const response = await fetch(`${window.API_BASE_URL}/api/instances/${id}/${action}`, { method: 'POST' });

            if (!response.ok) throw new Error(`Failed to ${action} instance`);

            await this.loadInstances();
            ToastSystem.show(`Instance ${action}ed successfully`, 'success');
        } catch (error) {
            console.error(`Error ${action}ing instance:`, error);
            ToastSystem.show(`Failed to ${action} instance`, 'error');
        }
    }

    editInstance(id) {
        const instance = this.instances.get(id);
        if (!instance) return;

        this.showModal('Edit Proxy Instance', instance);
    }

    deleteInstance(id) {
        const instance = this.instances.get(id);
        if (!instance) return;

        ModalSystem.confirm(
            'Delete Instance',
            `Are you sure you want to delete "${Utils.escapeHtml(instance.name)}"?`,
            async () => {
                try {
                    const response = await fetch(`${window.API_BASE_URL}/api/instances/${id}`, { method: 'DELETE' });

                    if (!response.ok) throw new Error('Failed to delete instance');

                    this.instances.delete(id);
                    this.renderInstances();
                    this.updateStats();
                    ToastSystem.show('Instance deleted successfully', 'success');
                } catch (error) {
                    console.error('Error deleting instance:', error);
                    ToastSystem.show('Failed to delete instance', 'error');
                }
            }
        );
    }

    async saveInstance() {
        try {
            const formData = this.getFormData();

            const url = this.editingId ? `${window.API_BASE_URL}/api/instances/${this.editingId}` : `${window.API_BASE_URL}/api/instances`;
            const method = this.editingId ? 'PUT' : 'POST';

            const response = await fetch(url, {
                method,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(formData)
            });

            if (!response.ok) throw new Error('Failed to save instance');

            await this.loadInstances();
            this.hideModal();
            ToastSystem.show(`Instance ${this.editingId ? 'updated' : 'created'} successfully`, 'success');
        } catch (error) {
            console.error('Error saving instance:', error);
            ToastSystem.show('Failed to save instance', 'error');
        }
    }

    getFormData() {
        const ipFilterType = document.getElementById('ipFilterType').value;
        const ipList = document.getElementById('ipList').value;

        const data = {
            name: document.getElementById('instanceName').value,
            listen_ip: document.getElementById('listenIp').value,
            listen_port: parseInt(document.getElementById('listenPort').value),
            dst_ip: document.getElementById('dstIp').value,
            dst_port: parseInt(document.getElementById('dstPort').value),
            protocol: document.getElementById('instanceProtocol').value,
            auto_start: document.getElementById('autoStart').checked,
            connect_timeout_secs: parseInt(document.getElementById('connectTimeout').value),
            idle_timeout_secs: parseInt(document.getElementById('idleTimeout').value),
            log_level: document.getElementById('logLevel').value
        };

        // Add IP filtering based on type
        if (ipFilterType === 'allow') {
            data.allow_list = ipList.split('\n').map(ip => ip.trim()).filter(ip => ip);
        } else if (ipFilterType === 'deny') {
            data.deny_list = ipList.split('\n').map(ip => ip.trim()).filter(ip => ip);
        }

        return data;
    }
}

// Global functions for onclick handlers
function showCreateModal() {
    proxyManager.showModal();
}


function hideModal() {
    proxyManager.hideModal();
}

function saveInstance() {
    proxyManager.saveInstance();
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    window.proxyManager = new VoidProxyManager();
});