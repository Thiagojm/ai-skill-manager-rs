<script lang="ts">
  export let name: 'sun' | 'moon' | 'search' | 'plus' | 'refresh' | 'copy' | 'more' | 'folder' | 'empty' | 'check' | 'alert' | 'info'
  export let size = 16

  const paths = {
    sun: 'M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4',
    moon: 'M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z',
    search: 'M11 4a7 7 0 1 0 0 14a7 7 0 1 0 0-14M20 20l-4-4',
    plus: 'M12 5v14M5 12h14',
    refresh: 'M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5',
    copy: 'M9 9h11v11H9zM5 15H4V4h11v1',
    more: 'M5 12h.01M12 12h.01M19 12h.01',
    folder: 'M3 6h6l2 2h10v11H3z',
    empty: 'M4 4h16v16H4zM4 14h5l1 2h4l1-2h5',
    check: 'M5 12l5 5L20 7',
    alert: 'M12 7v6M12 17h.01',
    info: 'M12 11v6M12 7h.01',
  }
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d={paths[name]} /></svg>
