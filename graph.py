import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime

# Option to toggle line connections
CONNECT_LINES = False
# Option to show both lines and points
SHOW_BOTH = True

# Load the data
df_client = pd.read_csv('steve-client.csv', names=['timestamp', 'predicted', 'received', 'smoothed', 'predicted_vel', 'received_vel'])
df_server = pd.read_csv('steve-server.csv', names=['timestamp', 'value', 'velocity'])

# Convert UNIX microsecond timestamps to datetime
df_client['timestamp'] = pd.to_datetime(df_client['timestamp'], unit='us')
df_server['timestamp'] = pd.to_datetime(df_server['timestamp'], unit='us')

# Plotting
plt.figure(figsize=(14, 10))
ax1 = plt.gca()  # primary axis
ax2 = ax1.twinx()  # secondary axis

# Determine plot types based on flags
if CONNECT_LINES:
    plot_type = '-'
elif SHOW_BOTH:
    plot_type = '-o'
else:
    plot_type = 'o'

# Primary axis for values
ax1.plot(df_client['timestamp'], df_client['predicted'], plot_type, label='Client Predicted', linewidth=2, color='blue')
ax1.plot(df_client['timestamp'], df_client['received'], plot_type, label='Client Received', linewidth=2, color='green')
ax1.plot(df_client['timestamp'], df_client['smoothed'], plot_type, label='Client Smoothed', linewidth=2, color='purple')
ax1.plot(df_server['timestamp'], df_server['value'], plot_type, label='Server', linewidth=2, color='red')

# Secondary axis for velocities
ax2.plot(df_client['timestamp'], df_client['predicted_vel'], plot_type, label='Client Predicted Vel (vel)', linewidth=2, color='cyan')
ax2.plot(df_client['timestamp'], df_client['received_vel'], plot_type, label='Client Received Vel (vel)', linewidth=2, color='magenta')
ax2.plot(df_server['timestamp'], df_server['velocity'], plot_type, label='Server Vel (vel)', linewidth=2, color='orange')

# Formatting the primary axis
ax1.set_xlabel('Time')
ax1.set_ylabel('Value')
ax1.set_title('Client and Server Values and Velocities Over Time')
ax1.legend(loc='upper left')
ax1.grid(True)

# Formatting the secondary axis
ax2.set_ylabel('Velocity')
ax2.legend(loc='upper right')

# Format x-axis with more readable date format
ax1.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d %H:%M:%S'))
ax1.xaxis.set_major_locator(mdates.MinuteLocator(interval=30))  # adjust the interval as needed
plt.gcf().autofmt_xdate()  # Rotation

# Show plot
plt.show()
