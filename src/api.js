import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

class TauriAPI {
  constructor() {
    this.baseURL = '' // Not needed for Tauri
    this.initFileWatcher()
  }

  async initFileWatcher() {
    try {
      await invoke('start_file_watcher')
      await listen('files-changed', () => {
        // Emit custom event that your frontend can listen to
        window.dispatchEvent(new CustomEvent('filesChanged'))
      })
    } catch (error) {
      console.error('Failed to initialize file watcher:', error)
    }
  }

  // Tags API
  async getTags(path) {
    try {
      return await invoke('get_tags', { path })
    } catch (error) {
      console.error('Error getting tags:', error)
      return {}
    }
  }

  async updateTagBackgroundColor(path, colors) {
    try {
      await invoke('update_tag_background_color', { path, colors })
    } catch (error) {
      console.error('Error updating tag colors:', error)
      throw error
    }
  }

  // Title API
  async getTitle() {
    try {
      return await invoke('get_title')
    } catch (error) {
      console.error('Error getting title:', error)
      return ''
    }
  }

  // Resource API
  async getResource(path) {
    try {
      return await invoke('get_resource', { path })
    } catch (error) {
      console.error('Error getting resource:', error)
      return []
    }
  }

  async createResource(path, isFile = false, content = '') {
    try {
      await invoke('create_resource', {
        path,
        isFile: isFile ? true : null,
        content: content || null
      })
    } catch (error) {
      console.error('Error creating resource:', error)
      throw error
    }
  }

  async updateResource(path, newPath = null, content = null) {
    try {
      await invoke('update_resource', {
        path,
        newPath: newPath || null,
        content: content || null
      })
    } catch (error) {
      console.error('Error updating resource:', error)
      throw error
    }
  }

  async deleteResource(path) {
    try {
      await invoke('delete_resource', { path })
    } catch (error) {
      console.error('Error deleting resource:', error)
      throw error
    }
  }

  // Image API
  async uploadImage(file) {
    try {
      // Convert file to array buffer, then to array
      const arrayBuffer = await file.arrayBuffer()
      const fileData = Array.from(new Uint8Array(arrayBuffer))

      return await invoke('upload_image', {
        fileData,
        filename: file.name
      })
    } catch (error) {
      console.error('Error uploading image:', error)
      throw error
    }
  }

  async getImage(filename) {
    try {
      const imageData = await invoke('get_image', { filename })
      // Convert array back to blob for frontend use
      const blob = new Blob([new Uint8Array(imageData)])
      return URL.createObjectURL(blob)
    } catch (error) {
      console.error('Error getting image:', error)
      throw error
    }
  }

  // Sort API
  async updateSort(path, sortData) {
    try {
      await invoke('update_sort', { path, sortData })
    } catch (error) {
      console.error('Error updating sort:', error)
      throw error
    }
  }

  async getSort(path) {
    try {
      return await invoke('get_sort', { path })
    } catch (error) {
      console.error('Error getting sort:', error)
      return {}
    }
  }

  // Compatibility methods for existing frontend code
  async get(endpoint) {
    const cleanEndpoint = endpoint.replace(/^\/+|\/+$/g, '')

    if (cleanEndpoint.startsWith('tags/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(5))
      return { json: () => this.getTags(path) }
    }

    if (cleanEndpoint === 'title') {
      return { json: () => this.getTitle() }
    }

    if (cleanEndpoint.startsWith('resource/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(9))
      return { json: () => this.getResource(path) }
    }

    if (cleanEndpoint.startsWith('sort/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(5))
      return { json: () => this.getSort(path) }
    }

    if (cleanEndpoint.startsWith('image/')) {
      const filename = cleanEndpoint.substring(6)
      const imageUrl = await this.getImage(filename)
      return { blob: () => fetch(imageUrl).then((r) => r.blob()) }
    }

    throw new Error(`Unknown GET endpoint: ${endpoint}`)
  }

  async post(endpoint, data) {
    const cleanEndpoint = endpoint.replace(/^\/+|\/+$/g, '')

    if (cleanEndpoint.startsWith('resource/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(9))
      await this.createResource(path, data.isFile, data.content)
      return { status: 201 }
    }

    if (cleanEndpoint === 'image') {
      const imageName = await this.uploadImage(data.get('file'))
      return { json: () => Promise.resolve(imageName) }
    }

    throw new Error(`Unknown POST endpoint: ${endpoint}`)
  }

  async patch(endpoint, data) {
    const cleanEndpoint = endpoint.replace(/^\/+|\/+$/g, '')

    if (cleanEndpoint.startsWith('tags/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(5))
      await this.updateTagBackgroundColor(path, data)
      return { status: 204 }
    }

    if (cleanEndpoint.startsWith('resource/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(9))
      await this.updateResource(path, data.newPath, data.content)
      return { status: 204 }
    }

    throw new Error(`Unknown PATCH endpoint: ${endpoint}`)
  }

  async put(endpoint, data) {
    const cleanEndpoint = endpoint.replace(/^\/+|\/+$/g, '')

    if (cleanEndpoint.startsWith('sort/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(5))
      await this.updateSort(path, data)
      return { status: 200 }
    }

    throw new Error(`Unknown PUT endpoint: ${endpoint}`)
  }

  async delete(endpoint) {
    const cleanEndpoint = endpoint.replace(/^\/+|\/+$/g, '')

    if (cleanEndpoint.startsWith('resource/')) {
      const path = decodeURIComponent(cleanEndpoint.substring(9))
      await this.deleteResource(path)
      return { status: 204 }
    }

    throw new Error(`Unknown DELETE endpoint: ${endpoint}`)
  }
}

// Create and export singleton instance
const api = new TauriAPI()
export default api
