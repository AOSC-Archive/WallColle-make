# Pack Manifest

- Name: <%- name %>
- Date: <%- date %>
- Entries: <% wallpapers.len() %>


Title | Contributor | License
------|-------------|--------
<% for wallpaper in &wallpapers { %><%- wallpaper.title %> | <%- wallpaper.artist %> | <%- wallpaper.license %><% } %>
