import time
import unittest
from unittest.mock import Mock, patch, MagicMock, call
import os
import tempfile
import logging
import urwid
from hypothesis import given, strategies as st
from yt_dlp.compat import shutil

from config_manager import ConfigManager
from logger import Logger
from notification_manager import NotificationManager
from performance_control import PerformanceControl
from tui_manager import TUIManager
from vpn_manager import VPNManager
from auxiliary_features import AuxiliaryFeatures
from error_handler import AutoYTDLPErrorHandler
from auto_ytdlp import AutoYTDLP


class TestConfigManager(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.config_file = os.path.join(self.temp_dir, 'test_config.toml')

    def tearDown(self):
        if os.path.exists(self.config_file):
            os.remove(self.config_file)
        os.rmdir(self.temp_dir)

    def test_load_config(self):
        with open(self.config_file, 'w') as f:
            f.write('[test]\nkey = "value"')

        config = ConfigManager(self.config_file)
        self.assertEqual(config.get('test', 'key'), 'value')

    def test_default_config(self):
        config = ConfigManager('non_existent_file.toml')
        self.assertIsNotNone(config.get('general', 'links_file'))

    def test_set_config(self):
        config = ConfigManager(self.config_file)
        config.set('test', 'new_key', 'new_value')
        self.assertEqual(config.get('test', 'new_key'), 'new_value')


class TestLogger(unittest.TestCase):
    def setUp(self):
        self.log_file = 'test.log'
        self.logger = Logger(self.log_file, level=logging.DEBUG)  # Set to DEBUG level

    def tearDown(self):
        if os.path.exists(self.log_file):
            os.remove(self.log_file)

    def test_log_levels(self):
        messages = ['info', 'warning', 'error', 'debug', 'critical']
        for level in messages:
            getattr(self.logger, level)(f'Test {level} message')

        with open(self.log_file, 'r') as f:
            log_content = f.read()

        for level in messages:
            self.assertIn(f'Test {level} message', log_content)


class TestNotificationManager(unittest.TestCase):
    @patch('plyer.notification.notify')
    def test_send_notification(self, mock_notify):
        nm = NotificationManager()
        nm.send_notification('Test Title', 'Test Message')
        mock_notify.assert_called_once_with(
            title='Test Title',
            message='Test Message',
            app_name='Auto-YTDLP',
            timeout=10
        )


class TestPerformanceControl(unittest.TestCase):
    def setUp(self):
        self.pc = PerformanceControl(max_concurrent_downloads=2, bandwidth_limit='5M')

    def test_speed_tracking(self):
        self.pc.start_time = time.time() - 10  # Simulate 10 seconds elapsed
        self.pc.downloaded_bytes = 1024 * 1024  # Simulate 1 MB downloaded

        # Simulate a progress update
        self.pc.progress_hook({
            'status': 'downloading',
            'downloaded_bytes': 1024 * 1024,
            'filename': 'test.mp4',
            '_percent_str': '50%'
        })

        # Check if speed is calculated correctly (should be around 102.4 KB/s)
        self.assertAlmostEqual(self.pc.get_current_speed(), 102.4, delta=1)

        # Simulate download completion
        self.pc.progress_hook({
            'status': 'finished',
            'filename': 'test.mp4'
        })

        # Check if values are reset
        self.assertEqual(self.pc.get_current_speed(), 0)
        self.assertIsNone(self.pc.start_time)
        self.assertEqual(self.pc.downloaded_bytes, 0)

    def test_queue_management(self):
        self.pc.add_to_queue('https://example.com/video1')
        self.pc.add_to_queue('https://example.com/video2')
        self.assertEqual(len(self.pc.download_queue), 2)

        self.pc.remove_from_queue('https://example.com/video1')
        self.assertEqual(len(self.pc.download_queue), 1)

    @patch('yt_dlp.YoutubeDL')
    def test_download_video(self, mock_ytdl):
        mock_ytdl.return_value.__enter__.return_value.extract_info.return_value = {'id': 'test_id'}
        result = self.pc.download_video('https://example.com/video')
        self.assertEqual(result['status'], 'success')


class TestTUIManager(unittest.TestCase):
    def setUp(self):
        self.tui = TUIManager(lambda: None, lambda: None)
        self.tui.main_loop = MagicMock()  # Mock the main_loop

    def test_add_download(self):
        self.tui.add_download('https://example.com/video')
        self.assertEqual(len(self.tui.download_list), 1)
        self.tui.main_loop.draw_screen.assert_called_once()

    def test_update_download_status(self):
        self.tui.add_download('https://example.com/video')
        self.tui.update_download_status('https://example.com/video', 'Completed')
        self.assertIn('Completed', self.tui.download_list[0].text)
        self.assertEqual(self.tui.main_loop.draw_screen.call_count, 2)


class TestVPNManager(unittest.TestCase):
    @patch('subprocess.run')
    def test_connect(self, mock_run):
        mock_run.return_value.stdout = 'Connected to VPN'
        vm = VPNManager()
        self.assertTrue(vm.connect())

    @patch('subprocess.run')
    def test_disconnect(self, mock_run):
        mock_run.return_value.stdout = 'Disconnected'
        vm = VPNManager()
        self.assertTrue(vm.disconnect())

    @patch('subprocess.run')
    def test_check_connection(self, mock_run):
        mock_run.return_value.stdout = 'Connected to Test Server'
        vm = VPNManager()
        status, location = vm.check_connection()
        self.assertTrue(status)
        self.assertEqual(location, 'Test Server')


class TestVPNManagerProperties(unittest.TestCase):
    @given(
        switch_after=st.integers(min_value=1, max_value=100),
        speed_threshold=st.integers(min_value=1, max_value=1000),
        current_speed=st.integers(min_value=0, max_value=2000),
        download_count=st.integers(min_value=0, max_value=200)
    )
    def test_should_switch_properties(self, switch_after, speed_threshold, current_speed, download_count):
        vpn_manager = VPNManager(switch_after=switch_after, speed_threshold=speed_threshold)

        # Simulate downloads
        for _ in range(download_count):
            vpn_manager.should_switch(current_speed)

        result = vpn_manager.should_switch(current_speed)

        # Property 1: If current_speed is below threshold, should always switch
        if current_speed < speed_threshold:
            assert result == True

        # Property 2: If download_count + 1 is a multiple of switch_after, should switch
        elif (download_count + 1) % switch_after == 0:
            assert result == True

        # Property 3: In all other cases, should not switch
        else:
            assert result == False


class TestAuxiliaryFeatures(unittest.TestCase):
    def setUp(self):
        self.aux = AuxiliaryFeatures({})

    @patch('subprocess.run')
    def test_auto_update_yt_dlp(self, mock_run):
        AuxiliaryFeatures.auto_update_yt_dlp()
        mock_run.assert_called_once()

    @patch('yt_dlp.YoutubeDL')
    def test_extract_metadata(self, mock_ytdl):
        mock_info = {'title': 'Test Video'}
        mock_ytdl.return_value.__enter__.return_value.extract_info.return_value = mock_info
        mock_ytdl.return_value.__enter__.return_value.sanitize_info.return_value = mock_info
        result = self.aux.extract_metadata('https://example.com/video')
        self.assertEqual(result['title'], 'Test Video')

    def test_utility_url_validation(self):
        self.assertTrue(self.aux.utility_url_validation('https://www.youtube.com/watch?v=dQw4w9WgXcQ'))
        self.assertFalse(self.aux.utility_url_validation('https://example.com/not_a_video'))


class TestErrorHandler(unittest.TestCase):
    def setUp(self):
        self.logger = Mock()
        self.error_handler = AutoYTDLPErrorHandler(self.logger)

    def test_handle_file_error(self):
        error = FileNotFoundError('test.txt')
        self.error_handler.handle_error(error)
        self.logger.error.assert_called_once_with('Unexpected error: test.txt')

    def test_handle_network_error(self):
        from urllib.error import URLError
        error = URLError('Network error')
        self.error_handler.handle_error(error)
        self.logger.error.assert_called_once_with('Network error: <urlopen error Network error>')


class TestAutoYTDLP(unittest.TestCase):
    @patch('auto_ytdlp.VPNManager')
    @patch('auto_ytdlp.TUIManager')
    @patch('auto_ytdlp.PerformanceControl')
    def setUp(self, mock_performance, mock_tui, mock_vpn):
        self.mock_performance = mock_performance.return_value
        self.mock_tui = mock_tui.return_value
        self.mock_vpn = mock_vpn.return_value

        self.auto_ytdlp = AutoYTDLP()
        self.auto_ytdlp.performance_control = self.mock_performance
        self.auto_ytdlp.tui_manager = self.mock_tui
        self.auto_ytdlp.vpn_manager = self.mock_vpn
        self.auto_ytdlp.tui_manager.main_loop = MagicMock()

    def test_load_url_list(self):
        with tempfile.NamedTemporaryFile(mode='w', delete=False) as temp_file:
            temp_file.write('https://example.com/video1\nhttps://example.com/video2')
            temp_file.flush()

            urls = self.auto_ytdlp.load_url_list(temp_file.name)
            self.assertEqual(len(urls), 2)
            self.assertIn('https://example.com/video1', urls)
            self.assertIn('https://example.com/video2', urls)

        os.unlink(temp_file.name)

    def test_start_downloads(self):
        self.mock_performance.process_queue.return_value = [
            {'status': 'success', 'url': 'https://example.com/video1'},
            {'status': 'error', 'url': 'https://example.com/video2', 'error': 'Download failed'}
        ]
        self.mock_performance.get_current_speed.return_value = 1000  # 1000 KB/s
        self.auto_ytdlp.start_downloads()

        self.mock_tui.update_download_status.assert_any_call('https://example.com/video1', 'Completed')
        self.mock_tui.update_download_status.assert_any_call('https://example.com/video2', 'Failed')
        self.mock_vpn.should_switch.assert_called_with(1000)


class TestAutoYTDLPIntegration(unittest.TestCase):
    @patch('auto_ytdlp.VPNManager')
    @patch('auto_ytdlp.TUIManager')
    @patch('auto_ytdlp.PerformanceControl')
    @patch('auto_ytdlp.ConfigManager')
    def setUp(self, mock_config, mock_performance, mock_tui, mock_vpn):
        self.mock_config = mock_config.return_value
        self.mock_performance = mock_performance.return_value
        self.mock_tui = mock_tui.return_value
        self.mock_vpn = mock_vpn.return_value

        self.temp_dir = tempfile.mkdtemp()
        self.log_file = os.path.join(self.temp_dir, 'test_log.txt')
        self.links_file = os.path.join(self.temp_dir, 'test_links.txt')

        self.mock_config.get.side_effect = lambda section, key, default=None: {
            ('general', 'log_file'): self.log_file,
            ('general', 'links_file'): self.links_file
        }.get((section, key), default)

        self.auto_ytdlp = AutoYTDLP()
        self.auto_ytdlp.performance_control = self.mock_performance
        self.auto_ytdlp.tui_manager = self.mock_tui
        self.auto_ytdlp.vpn_manager = self.mock_vpn

    def tearDown(self):
        shutil.rmtree(self.temp_dir)

    def test_full_download_process(self):
        urls = ["https://example.com/video1", "https://example.com/video2"]
        with open(self.links_file, 'w') as f:
            f.write('\n'.join(urls))

        self.mock_performance.process_queue.return_value = [
            {'status': 'success', 'url': urls[0]},
            {'status': 'success', 'url': urls[1]}
        ]
        self.mock_performance.get_current_speed.return_value = 500
        self.mock_vpn.should_switch.return_value = False

        self.auto_ytdlp.start_downloads()

        self.assertEqual(self.mock_tui.add_download.call_count, 2)
        self.mock_tui.update_download_status.assert_has_calls([
            call(urls[0], 'Completed'),
            call(urls[1], 'Completed')
        ], any_order=False)
        self.mock_vpn.switch_server.assert_not_called()
        self.assertEqual(self.mock_performance.process_queue.call_count, 1)

        print(f"process_queue call count: {self.mock_performance.process_queue.call_count}")
        print(f"VPN switch calls: {self.mock_vpn.switch_server.call_count}")
        print(f"TUI update calls: {self.mock_tui.update_download_status.call_args_list}")


class TestAutoYTDLPComprehensiveIntegration(unittest.TestCase):
    @patch('auto_ytdlp.VPNManager')
    @patch('auto_ytdlp.TUIManager')
    @patch('auto_ytdlp.PerformanceControl')
    @patch('auto_ytdlp.ConfigManager')
    def setUp(self, mock_config, mock_performance, mock_tui, mock_vpn):
        self.mock_config = mock_config.return_value
        self.mock_performance = mock_performance.return_value
        self.mock_tui = mock_tui.return_value
        self.mock_vpn = mock_vpn.return_value

        self.temp_dir = tempfile.mkdtemp()
        self.log_file = os.path.join(self.temp_dir, 'test_log.txt')
        self.links_file = os.path.join(self.temp_dir, 'test_links.txt')

        self.mock_config.get.side_effect = lambda section, key, default=None: {
            ('general', 'log_file'): self.log_file,
            ('general', 'links_file'): self.links_file
        }.get((section, key), default)

        self.auto_ytdlp = AutoYTDLP()
        self.auto_ytdlp.performance_control = self.mock_performance
        self.auto_ytdlp.tui_manager = self.mock_tui
        self.auto_ytdlp.vpn_manager = self.mock_vpn

    def tearDown(self):
        if os.path.exists(self.temp_dir):
            for file in os.listdir(self.temp_dir):
                os.remove(os.path.join(self.temp_dir, file))
            os.rmdir(self.temp_dir)

    def test_full_download_process_with_vpn_switch(self):
        urls = [
            "https://example.com/video1",
            "https://example.com/video2",
            "https://example.com/video3"
        ]
        with open(self.links_file, 'w') as f:
            f.write('\n'.join(urls))

        self.mock_performance.process_queue.side_effect = [
            [
                {'status': 'success', 'url': urls[0]},
                {'status': 'error', 'url': urls[1], 'error': 'Network error'},
                {'status': 'success', 'url': urls[2]}
            ],
            [
                {'status': 'success', 'url': urls[1]}
            ]
        ]
        self.mock_performance.get_current_speed.side_effect = [1000, 100, 800] * 10
        self.mock_vpn.should_switch.side_effect = [False, True, False] * 10

        self.auto_ytdlp.start_downloads()

        self.assertEqual(self.mock_tui.add_download.call_count, 3)
        self.mock_tui.update_download_status.assert_has_calls([
            call(urls[0], 'Completed'),
            call(urls[1], 'Failed'),
            call(urls[2], 'Completed'),
            call(urls[1], 'Completed')
        ], any_order=False)
        self.mock_vpn.switch_server.assert_called_once()
        self.assertEqual(self.mock_performance.process_queue.call_count, 2)

        print(f"process_queue call count: {self.mock_performance.process_queue.call_count}")
        print(f"VPN switch calls: {self.mock_vpn.switch_server.call_count}")
        print(f"TUI update calls: {self.mock_tui.update_download_status.call_args_list}")

    def test_error_handling_and_recovery(self):
        urls = ["https://example.com/video1"]
        with open(self.links_file, 'w') as f:
            f.write(urls[0])

        self.mock_performance.process_queue.side_effect = [
            [{'status': 'error', 'url': urls[0], 'error': 'Network error'}],
            [{'status': 'success', 'url': urls[0]}]
        ]
        self.mock_performance.get_current_speed.return_value = 100
        self.mock_vpn.should_switch.return_value = True

        self.auto_ytdlp.start_downloads()

        self.mock_vpn.switch_server.assert_called_once()
        self.assertEqual(self.mock_performance.process_queue.call_count, 2)
        self.mock_tui.update_download_status.assert_has_calls([
            call(urls[0], 'Failed'),
            call(urls[0], 'Completed')
        ], any_order=False)

        print(f"process_queue call count: {self.mock_performance.process_queue.call_count}")
        print(f"VPN switch calls: {self.mock_vpn.switch_server.call_count}")
        print(f"TUI update calls: {self.mock_tui.update_download_status.call_args_list}")


class TestErrorHandlingAndRecovery(unittest.TestCase):
    @patch('vpn_manager.VPNManager')
    @patch('tui_manager.TUIManager')
    def setUp(self, mock_tui, mock_vpn):
        self.auto_ytdlp = AutoYTDLP()
        self.mock_tui = mock_tui.return_value
        self.mock_vpn = mock_vpn.return_value
        self.auto_ytdlp.tui_manager = self.mock_tui
        self.auto_ytdlp.vpn_manager = self.mock_vpn

    @patch('performance_control.PerformanceControl')
    def test_recover_from_download_error(self, mock_performance):
        mock_performance.return_value.process_queue.return_value = [
            {'status': 'error', 'url': 'https://example.com/video1', 'error': 'Network error'},
            {'status': 'success', 'url': 'https://example.com/video2'},
        ]
        self.auto_ytdlp.performance_control = mock_performance.return_value
        self.auto_ytdlp.start_downloads()

        self.mock_tui.update_download_status.assert_any_call('https://example.com/video1', 'Failed')
        self.mock_tui.update_download_status.assert_any_call('https://example.com/video2', 'Completed')

    @patch('performance_control.PerformanceControl')
    def test_vpn_switch_on_slow_speed(self, mock_performance):
        self.mock_vpn.should_switch.return_value = True
        self.mock_vpn.switch_server.return_value = True

        mock_performance.return_value.process_queue.return_value = [
            {'status': 'success', 'url': 'https://example.com/video1'},
        ]
        self.auto_ytdlp.performance_control = mock_performance.return_value
        self.auto_ytdlp.start_downloads()

        self.mock_vpn.should_switch.assert_called()
        self.mock_vpn.switch_server.assert_called()


class TestConfigFileParsingEdgeCases(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.config_file = os.path.join(self.temp_dir, 'test_config.toml')

    def tearDown(self):
        if os.path.exists(self.config_file):
            os.remove(self.config_file)
        os.rmdir(self.temp_dir)

    def test_empty_config_file(self):
        open(self.config_file, 'w').close()  # Create an empty file
        config = ConfigManager(self.config_file)
        self.assertEqual(config.get('general', 'links_file'), 'links.txt')

    def test_malformed_toml(self):
        with open(self.config_file, 'w') as f:
            f.write('this is not valid TOML')
        config = ConfigManager(self.config_file)
        self.assertEqual(config.get('general', 'links_file'), 'links.txt')

    def test_missing_required_setting(self):
        with open(self.config_file, 'w') as f:
            f.write('[general]\n# links_file is missing')
        config = ConfigManager(self.config_file)
        self.assertEqual(config.get('general', 'links_file'), 'links.txt')


class TestTUIManagerExpanded(unittest.TestCase):
    def setUp(self):
        self.tui = TUIManager(lambda: None, lambda: None)
        self.tui.main_loop = MagicMock()
        self.tui.output_listbox = MagicMock()

    def test_create_main_widget(self):
        main_widget = self.tui.create_main_widget()
        self.assertIsInstance(main_widget, urwid.Frame)
        self.assertIsInstance(main_widget.body, urwid.Columns)
        self.assertIsInstance(main_widget.header, urwid.Text)
        self.assertIsInstance(main_widget.footer, urwid.Text)

    def test_handle_input_quit(self):
        with self.assertRaises(urwid.ExitMainLoop):
            self.tui.handle_input('q')

    def test_handle_input_start(self):
        self.tui.start_downloads = MagicMock()
        self.tui.handle_input('s')
        self.tui.start_downloads.assert_called_once()

    def test_handle_input_stop(self):
        self.tui.stop_downloads = MagicMock()
        self.tui.handle_input('x')
        self.tui.stop_downloads.assert_called_once()

    def test_add_download_updates_list(self):
        self.tui.add_download('https://example.com/video')
        self.assertEqual(len(self.tui.download_list), 1)
        self.assertIn('https://example.com/video', self.tui.download_list[0].text)

    def test_update_download_status(self):
        self.tui.add_download('https://example.com/video')
        self.tui.update_download_status('https://example.com/video', 'Completed')
        self.assertIn('Completed', self.tui.download_list[0].text)

    def test_show_output(self):
        self.tui.show_output('Test message')
        self.assertEqual(len(self.tui.output_list), 1)
        self.assertEqual(self.tui.output_list[0].text, 'Test message')
        self.assertEqual(self.tui.output_listbox.focus_position, len(self.tui.output_list) - 1)


class TestDownloadQueueManagement(unittest.TestCase):
    def setUp(self):
        self.pc = PerformanceControl(max_concurrent_downloads=2, bandwidth_limit='5M')

    def test_queue_order(self):
        self.pc.add_to_queue('https://example.com/video1')
        self.pc.add_to_queue('https://example.com/video2')
        self.pc.add_to_queue('https://example.com/video3')
        self.assertEqual(self.pc.download_queue, [
            'https://example.com/video1',
            'https://example.com/video2',
            'https://example.com/video3'
        ])

    def test_remove_from_middle_of_queue(self):
        self.pc.add_to_queue('https://example.com/video1')
        self.pc.add_to_queue('https://example.com/video2')
        self.pc.add_to_queue('https://example.com/video3')
        self.pc.remove_from_queue('https://example.com/video2')
        self.assertEqual(self.pc.download_queue, [
            'https://example.com/video1',
            'https://example.com/video3'
        ])

    @patch('yt_dlp.YoutubeDL')
    def test_concurrent_downloads(self, mock_ytdl):
        mock_ytdl.return_value.__enter__.return_value.extract_info.return_value = {'id': 'test_id'}
        self.pc.add_to_queue('https://example.com/video1')
        self.pc.add_to_queue('https://example.com/video2')
        self.pc.add_to_queue('https://example.com/video3')
        results = self.pc.process_queue()
        self.assertEqual(len(results), 3)
        self.assertEqual(mock_ytdl.call_count, 3)


class TestVPNSwitching(unittest.TestCase):
    def setUp(self):
        self.vpn = VPNManager(switch_after=2, speed_threshold=500)

    def test_should_switch_after_downloads(self):
        self.assertFalse(self.vpn.should_switch(1000))
        self.assertTrue(self.vpn.should_switch(1000))  # This will be the second call, so it should return True
        self.assertFalse(self.vpn.should_switch(1000))  # This resets the counter

    def test_should_switch_on_slow_speed(self):
        self.assertTrue(self.vpn.should_switch(400))

    @patch('subprocess.run')
    def test_switch_server(self, mock_run):
        mock_run.side_effect = [
            MagicMock(stdout='Disconnected'),
            MagicMock(stdout='Connected to VPN')
        ]
        self.assertTrue(self.vpn.switch_server())
        self.assertEqual(mock_run.call_count, 2)


if __name__ == '__main__':
    unittest.main()
